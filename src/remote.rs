use std::error::Error;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

const REMOTE_PREFIXES: &[&str] = &["https://", "http://", "git@"];

pub fn url(path: &Path) -> Option<&str> {
    let input = path.to_str()?;
    REMOTE_PREFIXES
        .iter()
        .any(|prefix| input.starts_with(prefix))
        .then_some(input)
}

/// Shallow clone into a temporary directory that is deleted when the returned value drops
pub fn clone(url: &str) -> Result<TempDir, Box<dyn Error>> {
    let checkout = tempfile::tempdir()?;
    eprintln!("Cloning {url}...");
    let output = Command::new("git")
        .args(["clone", "--depth", "1", "--quiet", url])
        .arg(checkout.path())
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cannot clone {url}: {}", message.trim()).into());
    }
    Ok(checkout)
}

pub fn repository_name(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    trimmed
        .rsplit(['/', ':'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("repository")
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    use super::{clone, repository_name, url};

    #[test]
    fn detects_remote_urls() {
        assert!(url(Path::new("https://github.com/user/repo")).is_some());
        assert!(url(Path::new("git@github.com:user/repo.git")).is_some());
        assert!(url(Path::new("./https-notes")).is_none());
    }

    #[test]
    fn derives_names_from_urls() {
        assert_eq!(repository_name("https://github.com/user/repo.git"), "repo");
        assert_eq!(repository_name("https://github.com/user/repo/"), "repo");
        assert_eq!(repository_name("git@github.com:repo.git"), "repo");
    }

    #[test]
    fn clones_into_a_temporary_directory() {
        let origin = tempfile::tempdir().unwrap();
        let path = origin.path();
        let git = |arguments: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(path)
                .args(arguments)
                .status();
            assert!(status.unwrap().success());
        };
        git(&["init", "-q"]);
        std::fs::write(path.join("lib.rs"), "fn a() {}").unwrap();
        git(&["add", "."]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "commit",
            "-qm",
            "init",
        ]);
        let checkout = clone(&format!("file://{}", path.display())).unwrap();
        assert!(checkout.path().join("lib.rs").is_file());
        let checkout_path = checkout.path().to_path_buf();
        drop(checkout);
        assert!(!checkout_path.exists());
    }
}
