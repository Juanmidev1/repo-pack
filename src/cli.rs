use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Preset {
    /// Changes against main plus the files that import them
    Review,
    /// Whole repository as skeleton with the top 10 heaviest files
    Arch,
    /// Uncommitted changes in full, files that import them as skeleton
    Bug,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Xml,
    Md,
}

/// Pack a local or remote repository into a single LLM-friendly text block
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct Args {
    /// Root directory of the repository to pack, or a git URL to clone temporarily
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

    /// Number of heaviest files listed in the summary [default: 3]
    #[arg(long, value_name = "N")]
    pub top: Option<usize>,

    /// Keep secrets such as API keys and passwords instead of redacting them
    #[arg(long)]
    pub allow_secrets: bool,

    /// Replace function bodies with placeholders, keeping signatures, types and imports
    #[arg(long)]
    pub skeleton: bool,

    /// Glob of files kept complete while the rest are skeletonized (repeatable, implies --skeleton)
    #[arg(long, value_name = "GLOB")]
    pub focus: Vec<String>,

    /// Only pack files changed against REF (default HEAD), including uncommitted and untracked ones
    #[arg(long, value_name = "REF", num_args = 0..=1, default_missing_value = "HEAD")]
    pub diff: Option<String>,

    /// With --diff, also pack the files that import the changed ones
    #[arg(long, requires = "diff")]
    pub impact: bool,

    /// Workflow preset; explicit flags take precedence
    #[arg(long, value_enum)]
    pub preset: Option<Preset>,
}

impl Args {
    pub fn with_preset(mut self) -> Self {
        match self.preset {
            Some(Preset::Review) => {
                self.diff.get_or_insert_with(|| "main".to_string());
                self.impact = true;
            }
            Some(Preset::Arch) => {
                self.skeleton = true;
                self.top.get_or_insert(10);
            }
            Some(Preset::Bug) => {
                self.diff.get_or_insert_with(|| "HEAD".to_string());
                self.impact = true;
            }
            None => {}
        }
        self
    }
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
    use clap::Parser;

    use super::{Args, parse_size};

    #[test]
    fn presets_fill_unset_flags_only() {
        let review = Args::parse_from(["repo-pack", "--preset", "review"]).with_preset();
        assert_eq!(review.diff.as_deref(), Some("main"));
        assert!(review.impact);
        let arch = Args::parse_from(["repo-pack", "--preset", "arch", "--top", "4"]).with_preset();
        assert!(arch.skeleton);
        assert_eq!(arch.top, Some(4));
        let bug = Args::parse_from(["repo-pack", "--preset", "bug", "--diff=dev"]).with_preset();
        assert_eq!(bug.diff.as_deref(), Some("dev"));
    }

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
