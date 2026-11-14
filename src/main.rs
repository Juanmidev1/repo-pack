mod cli;
mod formatter;
mod scanner;

use std::error::Error;
use std::fs;
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::Args;

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        // Piping into tools like `head` closes stdout early, which is not a failure
        Err(error) if is_broken_pipe(error.as_ref()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    if !args.path.is_dir() {
        return Err(format!("{} is not a directory", args.path.display()).into());
    }
    let files = scanner::scan(args)?;
    let packed = formatter::render(&files, &repository_name(&args.path), args.format);
    match &args.output {
        Some(output_path) => fs::write(output_path, packed)
            .map_err(|error| format!("cannot write {}: {error}", output_path.display()))?,
        None => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(packed.as_bytes())?;
            stdout.flush()?;
        }
    }
    Ok(())
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
