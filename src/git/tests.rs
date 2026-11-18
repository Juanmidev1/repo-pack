use std::fs;
use std::path::Path;
use std::process::Command;

use super::{changed_files, impacted_files, module_name};
use crate::scanner::SourceFile;

fn source(path: &str, content: &str) -> SourceFile {
    SourceFile {
        path: path.to_string(),
        content: content.to_string(),
        ..Default::default()
    }
}

#[test]
fn uses_folder_name_for_directory_entry_files() {
    assert_eq!(module_name("src/scanner/mod.rs"), Some("scanner"));
    assert_eq!(module_name("web/api/index.ts"), Some("api"));
    assert_eq!(module_name("pkg/utils/__init__.py"), Some("utils"));
    assert_eq!(module_name("src/tokens.rs"), Some("tokens"));
}

#[test]
fn finds_importers_across_languages() {
    let changed = vec!["src/scanner/mod.rs".to_string(), "web/api.ts".to_string()];
    let files = [
        source("src/main.rs", "use crate::scanner::{self, SourceFile};"),
        source("src/other.rs", "use crate::modding;"),
        source(
            "src/inline.rs",
            "mod tests {\n    let scanner = 1;\n    run();\n}",
        ),
        source("web/app.tsx", "import { get } from './api';"),
        source("web/legacy.js", "const api = require('./api');"),
        source("tools/run.py", "from pkg.scanner import scan"),
        source("src/scanner/mod.rs", "mod scanner;"),
    ];
    let impacted = impacted_files(&changed, &files);
    assert_eq!(
        impacted,
        [
            "src/main.rs",
            "web/app.tsx",
            "web/legacy.js",
            "tools/run.py"
        ]
    );
}

fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(arguments)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn lists_modified_and_untracked_files() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path();
    git(path, &["init", "-q"]);
    fs::write(path.join("kept.rs"), "fn a() {}").unwrap();
    fs::write(path.join("edited.rs"), "fn b() {}").unwrap();
    git(path, &["add", "."]);
    git(path, &["commit", "-qm", "init"]);
    fs::write(path.join("edited.rs"), "fn b() { 1; }").unwrap();
    fs::write(path.join("new.rs"), "fn c() {}").unwrap();
    fs::write(path.join("ñandú.rs"), "fn d() {}").unwrap();
    assert_eq!(
        changed_files(path, "HEAD").unwrap(),
        ["edited.rs", "new.rs", "ñandú.rs"]
    );
}

#[test]
fn rejects_refs_that_look_like_options() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    let target = root.path().join("written");
    let base = format!("--output={}", target.display());
    assert!(changed_files(root.path(), &base).is_err());
    assert!(!target.exists());
}

#[test]
fn reports_non_repositories() {
    let root = tempfile::tempdir().unwrap();
    let error = changed_files(root.path(), "HEAD").unwrap_err();
    assert!(
        error
            .to_string()
            .ends_with("is not inside a git repository")
    );
}
