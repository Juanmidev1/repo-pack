use colored::{ColoredString, Colorize};

use crate::pipeline::Package;
use crate::scanner::SourceFile;
use crate::tokens::{CONTEXT_WINDOWS, percentage_of, top_heaviest};

pub fn print(package: &Package, top: usize) {
    let files = &package.files;
    let total_tokens = package.tokens;
    let total_lines: usize = files.iter().map(|file| file.content.lines().count()).sum();
    let window_usage: Vec<String> = CONTEXT_WINDOWS
        .iter()
        .map(|(label, size)| {
            format!(
                "{label} {}",
                colorize_usage(percentage_of(total_tokens, *size))
            )
        })
        .collect();
    eprintln!("{}", "repo-pack summary".bold());
    eprintln!("  Files:   {}", files.len());
    eprintln!("  Lines:   {total_lines}");
    eprintln!("  Tokens:  {}", total_tokens.to_string().bold());
    eprintln!("  Context: {}", window_usage.join(" | "));
    print_savings(files);
    print_heaviest(files, total_tokens, top);
}

fn print_savings(files: &[SourceFile]) {
    let skeletonized = files.iter().filter(|file| file.skeleton).count();
    if skeletonized == 0 {
        return;
    }
    let original: usize = files.iter().map(|file| file.original_tokens).sum();
    let current: usize = files.iter().map(|file| file.tokens).sum();
    let saved = original.saturating_sub(current);
    eprintln!(
        "  Saved:   {} tokens ({:.1}%) in {skeletonized} skeleton file(s)",
        saved.to_string().green().bold(),
        percentage_of(saved, original)
    );
}

fn print_heaviest(files: &[SourceFile], total_tokens: usize, top: usize) {
    if top == 0 {
        return;
    }
    eprintln!("  Heaviest files:");
    for (rank, file) in top_heaviest(files, top).iter().enumerate() {
        let share = percentage_of(file.tokens, total_tokens);
        eprintln!(
            "    {}. {} - {} tokens ({share:.1}%)",
            rank + 1,
            file.path.cyan(),
            file.tokens
        );
    }
}

pub fn print_redactions(redactions: &[(String, usize)]) {
    if redactions.is_empty() {
        return;
    }
    let total: usize = redactions.iter().map(|(_, count)| count).sum();
    eprintln!(
        "{} redacted {total} secret(s), use --allow-secrets to keep them:",
        "warning:".yellow().bold()
    );
    for (path, count) in redactions {
        eprintln!("    {} ({count})", path.cyan());
    }
}

fn colorize_usage(percentage: f64) -> ColoredString {
    let text = format!("{percentage:.1}%");
    match percentage {
        value if value > 100.0 => text.red().bold(),
        value if value > 75.0 => text.yellow(),
        _ => text.green(),
    }
}
