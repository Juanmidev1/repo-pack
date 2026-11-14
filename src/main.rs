mod cli;
mod clipboard;
mod formatter;
mod scanner;
mod tokens;

use std::error::Error;
use std::fs;
use std::io::{self, ErrorKind, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use colored::{ColoredString, Colorize};

use crate::cli::Args;
use crate::scanner::SourceFile;
use crate::tokens::{CONTEXT_WINDOWS, percentage_of, top_heaviest};

const HEAVIEST_FILES_SHOWN: usize = 3;

fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    if std::env::var_os(clipboard::DAEMON_ENV).is_some() {
        return match clipboard::serve_from_stdin() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }
    // Keep escape codes out of logs and redirected stderr
    if !io::stderr().is_terminal() {
        colored::control::set_override(false);
    }
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        // Piping into tools like `head` closes stdout early, which is not a failure
        Err(error) if is_broken_pipe(error.as_ref()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{} {error}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    if !args.path.is_dir() {
        return Err(format!("{} is not a directory", args.path.display()).into());
    }
    let files = scanner::scan(args)?;
    let name = repository_name(&args.path);
    let packed = formatter::render(&files, &name, args.format);
    let total_tokens = tokens::estimate_tokens(&packed);
    if args.stats {
        eprint!("{}", formatter::render_tree(&files, &name));
    }
    print_summary(&files, total_tokens);
    if let Some(limit) = args.max_tokens
        && total_tokens > limit
    {
        return Err(
            format!("package has {total_tokens} tokens, above --max-tokens {limit}").into(),
        );
    }
    if !args.stats {
        write_output(args, &packed)?;
    }
    Ok(())
}

fn write_output(args: &Args, packed: &str) -> Result<(), Box<dyn Error>> {
    if args.copy {
        clipboard::copy(packed).map_err(|error| format!("cannot copy to clipboard: {error}"))?;
        eprintln!("{}", "Copied to clipboard".green());
    }
    if let Some(path) = &args.output {
        fs::write(path, packed)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    } else if !args.copy {
        let mut stdout = io::stdout().lock();
        stdout.write_all(packed.as_bytes())?;
        stdout.flush()?;
    }
    Ok(())
}

fn print_summary(files: &[SourceFile], total_tokens: usize) {
    let total_lines: usize = files.iter().map(|file| file.content.lines().count()).sum();
    let window_usage: Vec<String> = CONTEXT_WINDOWS
        .iter()
        .map(|(label, size)| {
            format!(
                "{label} {}",
                colorize_usage(percentage_of(total_tokens, *size))
            )
        })
        .collect();
    eprintln!("{}", "repo-pack summary".bold());
    eprintln!("  Files:   {}", files.len());
    eprintln!("  Lines:   {total_lines}");
    eprintln!("  Tokens:  {}", total_tokens.to_string().bold());
    eprintln!("  Context: {}", window_usage.join(" | "));
    eprintln!("  Heaviest files:");
    for (rank, file) in top_heaviest(files, HEAVIEST_FILES_SHOWN).iter().enumerate() {
        let share = percentage_of(file.tokens, total_tokens);
        eprintln!(
            "    {}. {} - {} tokens ({share:.1}%)",
            rank + 1,
            file.path.cyan(),
            file.tokens
        );
    }
}

fn colorize_usage(percentage: f64) -> ColoredString {
    let text = format!("{percentage:.1}%");
    match percentage {
        value if value > 100.0 => text.red().bold(),
        value if value > 75.0 => text.yellow(),
        _ => text.green(),
    }
}

fn repository_name(path: &Path) -> String {
    path.canonicalize()
        .ok()
        .and_then(|absolute| {
            absolute
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "repository".to_string())
}

fn is_broken_pipe(error: &(dyn Error + 'static)) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|io_error| io_error.kind() == ErrorKind::BrokenPipe)
}
