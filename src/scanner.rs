use std::fs;
use std::path::Path;

use ignore::WalkBuilder;
use ignore::overrides::{Override, OverrideBuilder};

use crate::cli::Args;
use crate::tokens::estimate_tokens;

const ALWAYS_EXCLUDED: &[&str] = &[".git/", "node_modules/", "target/"];
const LOCKFILES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "uv.lock",
    "poetry.lock",
    "composer.lock",
    "Gemfile.lock",
];
// Same heuristic as git: a NUL byte near the start means the file is binary
const BINARY_SNIFF_BYTES: usize = 8000;

#[derive(Default)]
pub struct SourceFile {
    pub path: String,
    pub content: String,
    pub tokens: usize,
    /// Tokens before any transformation, used to report what skeleton mode saved
    pub original_tokens: usize,
    pub skeleton: bool,
}

pub fn scan(args: &Args) -> Result<Vec<SourceFile>, ignore::Error> {
    let walker = WalkBuilder::new(&args.path)
        .overrides(build_overrides(args)?)
        // Respect .gitignore even when the folder is not a git repository
        .require_git(false)
        .sort_by_file_path(|left, right| left.cmp(right))
        .build();
    let mut files = Vec::new();
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("warning: {error}");
                continue;
            }
        };
        let is_file = entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file());
        if is_file && passes_filters(entry.path(), args) {
            files.extend(read_text_file(&args.path, entry.path()));
        }
    }
    Ok(files)
}

fn build_overrides(args: &Args) -> Result<Override, ignore::Error> {
    let mut builder = OverrideBuilder::new(&args.path);
    let user_patterns = args.exclude.iter().cloned();
    let output_pattern = output_exclusion(&args.path, args.output.as_deref());
    let patterns = ALWAYS_EXCLUDED
        .iter()
        .map(|pattern| pattern.to_string())
        .chain(user_patterns)
        .chain(output_pattern);
    for pattern in patterns {
        builder.add(&format!("!{pattern}"))?;
    }
    builder.build()
}

// A previous `-o context.xml` inside the repo would otherwise be packed into the next run
fn output_exclusion(root: &Path, output: Option<&Path>) -> Option<String> {
    let output = output?;
    let parent = match output.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let absolute_output = parent.canonicalize().ok()?.join(output.file_name()?);
    let relative_output = absolute_output
        .strip_prefix(root.canonicalize().ok()?)
        .ok()?;
    Some(format!("/{}", relative_output.to_string_lossy()))
}

fn passes_filters(path: &Path, args: &Args) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !args.include_lockfiles && LOCKFILES.contains(&file_name) {
        return false;
    }
    if !matches_extension(path, &args.extensions) {
        return false;
    }
    args.max_size
        .is_none_or(|limit| fs::metadata(path).is_ok_and(|metadata| metadata.len() <= limit))
}

fn matches_extension(path: &Path, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    extensions.iter().any(|wanted| {
        wanted
            .trim_start_matches('.')
            .eq_ignore_ascii_case(extension)
    })
}

fn read_text_file(root: &Path, path: &Path) -> Option<SourceFile> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("warning: cannot read {}: {error}", path.display());
            return None;
        }
    };
    if bytes.iter().take(BINARY_SNIFF_BYTES).any(|&byte| byte == 0) {
        return None;
    }
    let content = String::from_utf8(bytes).ok()?;
    let relative_path = path.strip_prefix(root).unwrap_or(path);
    let tokens = estimate_tokens(&content);
    Some(SourceFile {
        path: relative_path.to_string_lossy().replace('\\', "/"),
        tokens,
        original_tokens: tokens,
        content,
        skeleton: false,
    })
}

#[cfg(test)]
mod tests;
