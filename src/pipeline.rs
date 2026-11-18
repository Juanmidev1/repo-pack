use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use colored::Colorize;

use crate::cli::{Args, Preset};
use crate::scanner::{self, SourceFile};
use crate::tokens;
use crate::{clipboard, formatter, git, remote, secrets, skeleton, summary};

const DEFAULT_TOP: usize = 3;

pub struct Package {
    pub files: Vec<SourceFile>,
    pub name: String,
    pub text: String,
    pub tokens: usize,
    /// Files with redacted secrets and how many were replaced in each
    pub redactions: Vec<(String, usize)>,
}

// Never writes to stdout, so callers that own it (like the MCP server) can reuse it
pub fn pack(args: &Args) -> Result<Package, Box<dyn Error>> {
    let Some(url) = remote::url(&args.path) else {
        return pack_local(args, repository_name(&args.path));
    };
    if args.diff.is_some() {
        return Err("--diff needs a local repository, not a URL".into());
    }
    let checkout = remote::clone(url)?;
    let local_args = Args {
        path: checkout.path().to_path_buf(),
        ..args.clone()
    };
    pack_local(&local_args, remote::repository_name(url))
}

fn pack_local(args: &Args, name: String) -> Result<Package, Box<dyn Error>> {
    if !args.path.is_dir() {
        return Err(format!("{} is not a directory", args.path.display()).into());
    }
    let mut files = scanner::scan(args)?;
    // Skeleton first so redaction warnings only cover code that is actually emitted
    let focus = narrow_to_git_scope(args, &mut files)?;
    skeleton::apply(&mut files, &args.path, args.skeleton, &focus)?;
    let redactions = if args.allow_secrets {
        Vec::new()
    } else {
        redact_secrets(&mut files)
    };
    let text = formatter::render(&files, &name, args.format);
    let tokens = tokens::estimate_tokens(&text);
    Ok(Package {
        files,
        name,
        text,
        tokens,
        redactions,
    })
}

// Returns the focus globs, since the bug preset focuses on whatever changed
fn narrow_to_git_scope(
    args: &Args,
    files: &mut Vec<SourceFile>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let Some(base) = &args.diff else {
        return Ok(args.focus.clone());
    };
    let changed = git::changed_files(&args.path, base)?;
    let mut scope: HashSet<String> = changed.iter().cloned().collect();
    if args.impact {
        scope.extend(git::impacted_files(&changed, files));
    }
    files.retain(|file| scope.contains(&file.path));
    if args.preset == Some(Preset::Bug) && args.focus.is_empty() {
        return Ok(changed.iter().map(|path| format!("/{path}")).collect());
    }
    Ok(args.focus.clone())
}

fn redact_secrets(files: &mut [SourceFile]) -> Vec<(String, usize)> {
    let mut redactions = Vec::new();
    for file in files {
        let (content, count) = secrets::redact(&file.content);
        if count > 0 {
            file.tokens = tokens::estimate_tokens(&content);
            file.content = content;
            redactions.push((file.path.clone(), count));
        }
    }
    redactions
}

pub fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    let package = pack(args)?;
    if args.stats {
        eprint!("{}", formatter::render_tree(&package.files, &package.name));
    }
    summary::print(&package, args.top.unwrap_or(DEFAULT_TOP));
    summary::print_redactions(&package.redactions);
    if let Some(limit) = args.max_tokens
        && package.tokens > limit
    {
        let total = package.tokens;
        return Err(format!("package has {total} tokens, above --max-tokens {limit}").into());
    }
    if !args.stats {
        write_output(args, &package.text)?;
    }
    Ok(())
}

fn write_output(args: &Args, packed: &str) -> Result<(), Box<dyn Error>> {
    if args.copy {
        clipboard::copy(packed).map_err(|error| format!("cannot copy to clipboard: {error}"))?;
        eprintln!("{}", "Copied to clipboard".green());
    }
    if let Some(path) = &args.output {
        fs::write(path, packed)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    } else if !args.copy {
        let mut stdout = io::stdout().lock();
        stdout.write_all(packed.as_bytes())?;
        stdout.flush()?;
    }
    Ok(())
}

fn repository_name(path: &Path) -> String {
    path.canonicalize()
        .ok()
        .and_then(|absolute| {
            absolute
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "repository".to_string())
}
