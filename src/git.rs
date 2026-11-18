use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

use crate::scanner::SourceFile;

// Covers Rust `use`/`mod`, ES `import`/`export ... from`, `require()` and Python imports
static IMPORT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:use\s+([^;]+);|mod\s+(\w+))",
        r#"(?:import|export)\s[^'"]*?from\s*['"]([^'"]+)['"]"#,
        r#"(?:import|require)\s*\(?\s*['"]([^'"]+)['"]"#,
        r"(?m)^\s*from\s+([\w.]+)\s+import\s+([\w., ()]+)",
        r"(?m)^\s*import\s+([\w., ]+)$",
    ]
    .into_iter()
    .map(|pattern| Regex::new(pattern).expect("import patterns are valid"))
    .collect()
});
// Entry files are imported through their folder name, not through their own stem
const DIRECTORY_ENTRY_STEMS: &[&str] = &["mod", "index", "__init__"];

/// Files changed against `base` (via its merge base), including uncommitted and untracked ones
pub fn changed_files(root: &Path, base: &str) -> Result<Vec<String>, Box<dyn Error>> {
    // A ref like `--output=file` would otherwise be parsed by git as an option
    if base.starts_with('-') {
        return Err(format!("invalid git ref: {base}").into());
    }
    // Outside a repository `git diff` falls back to --no-index and prints its whole usage
    run_git(root, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|_| format!("{} is not inside a git repository", root.display()))?;
    let mut changed: BTreeSet<String> = BTreeSet::new();
    let diff = [
        "diff",
        "--name-only",
        "--relative",
        "--merge-base",
        "--end-of-options",
        base,
    ];
    changed.extend(run_git(root, &diff)?);
    changed.extend(run_git(
        root,
        &["ls-files", "--others", "--exclude-standard"],
    )?);
    Ok(changed.into_iter().collect())
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        // Quoted paths like "\303\261.rs" would never match the scanned ones
        .args(["-c", "core.quotePath=false"])
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {} failed: {}", arguments[0], message.trim()).into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(str::to_string).collect())
}

/// Files that import any of the changed files, one level deep
pub fn impacted_files(changed: &[String], files: &[SourceFile]) -> Vec<String> {
    let modules: BTreeSet<&str> = changed
        .iter()
        .filter_map(|path| module_name(path))
        .collect();
    files
        .iter()
        .filter(|file| !changed.contains(&file.path))
        .filter(|file| imported_names(&file.content).any(|name| modules.contains(name)))
        .map(|file| file.path.clone())
        .collect()
}

fn module_name(path: &str) -> Option<&str> {
    let mut segments = path.rsplit('/');
    let file_name = segments.next()?;
    let stem = file_name
        .split_once('.')
        .map_or(file_name, |(stem, _)| stem);
    if DIRECTORY_ENTRY_STEMS.contains(&stem) {
        return segments.next();
    }
    Some(stem)
}

fn imported_names(content: &str) -> impl Iterator<Item = &str> {
    IMPORT_PATTERNS
        .iter()
        .flat_map(move |pattern| pattern.captures_iter(content))
        .flat_map(|captures| {
            (1..captures.len())
                .filter_map(move |index| captures.get(index))
                .map(|group| group.as_str())
                .collect::<Vec<_>>()
        })
        .flat_map(|specifier| {
            specifier.split(|character: char| !character.is_alphanumeric() && character != '_')
        })
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests;
