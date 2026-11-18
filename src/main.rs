mod cli;
mod clipboard;
mod formatter;
mod git;
mod mcp;
mod pipeline;
mod remote;
mod scanner;
mod secrets;
mod skeleton;
mod summary;
mod tokens;

use std::error::Error;
use std::io::{self, ErrorKind, IsTerminal};
use std::process::ExitCode;

use clap::Parser;
use colored::Colorize;

use crate::cli::Args;

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
    let args = Args::parse().with_preset();
    let outcome = if args.mcp {
        mcp::serve(&args.path)
    } else {
        pipeline::run(&args)
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        // Piping into tools like `head` closes stdout early, which is not a failure
        Err(error) if is_broken_pipe(error.as_ref()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{} {error}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}

fn is_broken_pipe(error: &(dyn Error + 'static)) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|io_error| io_error.kind() == ErrorKind::BrokenPipe)
}
