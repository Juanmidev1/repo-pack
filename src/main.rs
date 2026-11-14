mod cli;
mod scanner;

use std::io::{self, ErrorKind, Write};
use std::process::ExitCode;

use clap::Parser;

use crate::scanner::SourceFile;

fn main() -> ExitCode {
    let args = cli::Args::parse();
    if !args.path.is_dir() {
        eprintln!("error: {} is not a directory", args.path.display());
        return ExitCode::FAILURE;
    }
    let files = match scanner::scan(&args) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    match write_plain(&files) {
        // Piping into tools like `head` closes stdout early, which is not a failure
        Err(error) if error.kind() != ErrorKind::BrokenPipe => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
        _ => ExitCode::SUCCESS,
    }
}

// TODO: replace with the XML and Markdown formatters (feat/formatters)
fn write_plain(files: &[SourceFile]) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    for file in files {
        writeln!(stdout, "=== {} ===\n{}\n", file.path, file.content)?;
    }
    stdout.flush()
}
