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

#[test]
fn excludes_previous_output_file() {
    let root = create_fixture();
    let output_path = root.path().join("context.xml");
    fs::write(&output_path, "<repository/>").unwrap();
    let args = Args {
        output: Some(output_path),
        ..default_args(root.path())
    };
    assert!(!scanned_paths(&args).contains(&"context.xml".to_string()));
}
