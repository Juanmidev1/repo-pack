use std::fs;
use std::path::Path;

use ignore::WalkBuilder;
use ignore::overrides::{Override, OverrideBuilder};

use crate::cli::Args;

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

pub struct SourceFile {
    pub path: String,
    pub content: String,
}

pub fn scan(args: &Args) -> Result<Vec<SourceFile>, ignore::Error> {
    let walker = WalkBuilder::new(&args.path)
        .overrides(build_overrides(&args.path, &args.exclude)?)
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

fn build_overrides(root: &Path, user_excludes: &[String]) -> Result<Override, ignore::Error> {
    let mut builder = OverrideBuilder::new(root);
    let user_patterns = user_excludes.iter().map(String::as_str);
    for pattern in ALWAYS_EXCLUDED.iter().copied().chain(user_patterns) {
        builder.add(&format!("!{pattern}"))?;
    }
    builder.build()
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
    Some(SourceFile {
        path: relative_path.to_string_lossy().replace('\\', "/"),
        content,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use clap::Parser;
    use tempfile::TempDir;

    use super::scan;
    use crate::cli::Args;

    fn create_fixture() -> TempDir {
        let root = tempfile::tempdir().unwrap();
        let files: &[(&str, &[u8])] = &[
            (".gitignore", b"ignored.txt\n"),
            ("ignored.txt", b"secret"),
            ("src/main.rs", b"fn main() {}"),
            ("src/lib.rs", b"pub fn lib() {}"),
            ("tests/integration.rs", b"#[test] fn it() {}"),
            ("README.md", b"# Fixture"),
            ("Cargo.lock", b"lock"),
            ("image.png", b"\x89PNG\x00\x00binary"),
            ("node_modules/pkg/index.js", b"module.exports = 1"),
            ("big.txt", &[b'a'; 2048]),
        ];
        for (relative_path, content) in files {
            let path = root.path().join(relative_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        root
    }

    fn default_args(root: &Path) -> Args {
        Args::parse_from(["repo-pack".as_ref(), root.as_os_str()])
    }

    fn scanned_paths(args: &Args) -> Vec<String> {
        scan(args)
            .unwrap()
            .into_iter()
            .map(|file| file.path)
            .collect()
    }

    #[test]
    fn applies_default_exclusions() {
        let root = create_fixture();
        let paths = scanned_paths(&default_args(root.path()));
        assert_eq!(
            paths,
            [
                "README.md",
                "big.txt",
                "src/lib.rs",
                "src/main.rs",
                "tests/integration.rs"
            ]
        );
    }

    #[test]
    fn filters_by_extension_and_exclude_glob() {
        let root = create_fixture();
        let args = Args {
            extensions: vec!["rs".into()],
            exclude: vec!["tests/*".into()],
            ..default_args(root.path())
        };
        assert_eq!(scanned_paths(&args), ["src/lib.rs", "src/main.rs"]);
    }

    #[test]
    fn honors_max_size_and_lockfile_flag() {
        let root = create_fixture();
        let args = Args {
            max_size: Some(1024),
            include_lockfiles: true,
            ..default_args(root.path())
        };
        let paths = scanned_paths(&args);
        assert!(paths.contains(&"Cargo.lock".to_string()));
        assert!(!paths.contains(&"big.txt".to_string()));
    }
}
