use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Xml,
    Md,
}

/// Pack a local repository into a single LLM-friendly text block
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Root directory of the repository to pack
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Only include files with these extensions (comma separated, e.g. rs,ts,py)
    #[arg(short = 'e', long = "ext", value_delimiter = ',')]
    pub extensions: Vec<String>,

    /// Glob pattern to exclude, relative to the root (repeatable, e.g. "tests/*")
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Skip files larger than this size (e.g. 500, 100KB, 2MB)
    #[arg(long, value_parser = parse_size)]
    pub max_size: Option<u64>,

    /// Include lockfiles such as Cargo.lock or package-lock.json
    #[arg(long)]
    pub include_lockfiles: bool,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Xml)]
    pub format: OutputFormat,

    /// Write the result to this file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Copy the result to the system clipboard instead of printing it
    #[arg(short, long)]
    pub copy: bool,

    /// Abort without writing anything if the package exceeds this many tokens
    #[arg(long)]
    pub max_tokens: Option<usize>,

    /// Dry run: print the tree and token statistics without producing output
    #[arg(long)]
    pub stats: bool,
}

pub fn parse_size(input: &str) -> Result<u64, String> {
    let normalized = input.trim().to_ascii_uppercase();
    let digits_end = normalized
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(normalized.len());
    let (number, unit) = normalized.split_at(digits_end);
    let value: u64 = number
        .parse()
        .map_err(|_| format!("invalid size: {input}"))?;
    let multiplier = match unit.trim() {
        "" | "B" => 1,
        "K" | "KB" => 1024,
        "M" | "MB" => 1024 * 1024,
        _ => return Err(format!("unknown size unit in: {input}")),
    };
    value
        .checked_mul(multiplier)
        .ok_or_else(|| format!("size too large: {input}"))
}

#[cfg(test)]
mod tests {
    use super::parse_size;

    #[test]
    fn parses_sizes_with_and_without_units() {
        assert_eq!(parse_size("500"), Ok(500));
        assert_eq!(parse_size("100KB"), Ok(100 * 1024));
        assert_eq!(parse_size("2mb"), Ok(2 * 1024 * 1024));
    }

    #[test]
    fn rejects_invalid_sizes() {
        assert!(parse_size("KB").is_err());
        assert!(parse_size("10GB").is_err());
        assert!(parse_size("99999999999999999MB").is_err());
    }
}
