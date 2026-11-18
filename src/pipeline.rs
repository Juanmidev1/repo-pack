use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use colored::{ColoredString, Colorize};

use crate::cli::Args;
use crate::scanner::{self, SourceFile};
use crate::tokens::{self, CONTEXT_WINDOWS, percentage_of, top_heaviest};
use crate::{clipboard, formatter, secrets, skeleton};

pub struct Package {
    pub files: Vec<SourceFile>,
    pub name: String,
    pub text: String,
    pub tokens: usize,
    /// Files with redacted secrets and how many were replaced in each
    pub redactions: Vec<(String, usize)>,
}

// Side-effect free so callers that own stdout (like the MCP server) can reuse it
pub fn pack(args: &Args) -> Result<Package, Box<dyn Error>> {
    if !args.path.is_dir() {
        return Err(format!("{} is not a directory", args.path.display()).into());
    }
    let mut files = scanner::scan(args)?;
    // Skeleton first so redaction warnings only cover code that is actually emitted
    skeleton::apply(&mut files, &args.path, args.skeleton, &args.focus)?;
    let redactions = if args.allow_secrets {
        Vec::new()
    } else {
        redact_secrets(&mut files)
    };
    let name = repository_name(&args.path);
    let text = formatter::render(&files, &name, args.format);
    let tokens = tokens::estimate_tokens(&text);
    Ok(Package {
        files,
        name,
        text,
        tokens,
        redactions,
    })
}

fn redact_secrets(files: &mut [SourceFile]) -> Vec<(String, usize)> {
    let mut redactions = Vec::new();
    for file in files {
        let (content, count) = secrets::redact(&file.content);
        if count > 0 {
            file.tokens = tokens::estimate_tokens(&content);
            file.content = content;
            redactions.push((file.path.clone(), count));
        }
    }
    redactions
}

pub fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    let package = pack(args)?;
    if args.stats {
        eprint!("{}", formatter::render_tree(&package.files, &package.name));
    }
    print_summary(&package, args.top);
    print_redactions(&package.redactions);
    if let Some(limit) = args.max_tokens
        && package.tokens > limit
    {
        let total = package.tokens;
        return Err(format!("package has {total} tokens, above --max-tokens {limit}").into());
    }
    if !args.stats {
        write_output(args, &package.text)?;
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

fn print_summary(package: &Package, top: usize) {
    let files = &package.files;
    let total_tokens = package.tokens;
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
    print_savings(files);
    print_heaviest(files, total_tokens, top);
}

fn print_savings(files: &[SourceFile]) {
    let skeletonized = files.iter().filter(|file| file.skeleton).count();
    if skeletonized == 0 {
        return;
    }
    let original: usize = files.iter().map(|file| file.original_tokens).sum();
    let current: usize = files.iter().map(|file| file.tokens).sum();
    let saved = original.saturating_sub(current);
    eprintln!(
        "  Saved:   {} tokens ({:.1}%) in {skeletonized} skeleton file(s)",
        saved.to_string().green().bold(),
        percentage_of(saved, original)
    );
}

fn print_heaviest(files: &[SourceFile], total_tokens: usize, top: usize) {
    if top == 0 {
        return;
    }
    eprintln!("  Heaviest files:");
    for (rank, file) in top_heaviest(files, top).iter().enumerate() {
        let share = percentage_of(file.tokens, total_tokens);
        eprintln!(
            "    {}. {} - {} tokens ({share:.1}%)",
            rank + 1,
            file.path.cyan(),
            file.tokens
        );
    }
}

fn print_redactions(redactions: &[(String, usize)]) {
    if redactions.is_empty() {
        return;
    }
    let total: usize = redactions.iter().map(|(_, count)| count).sum();
    eprintln!(
        "{} redacted {total} secret(s), use --allow-secrets to keep them:",
        "warning:".yellow().bold()
    );
    for (path, count) in redactions {
        eprintln!("    {} ({count})", path.cyan());
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
