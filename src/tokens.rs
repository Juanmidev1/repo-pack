use std::cmp::Reverse;

use crate::scanner::SourceFile;

// Calibrated for source code: symbols and short identifiers make it denser than prose (~4.0)
const CHARS_PER_TOKEN: f64 = 3.8;

pub const CONTEXT_WINDOWS: [(&str, usize); 3] =
    [("128k", 128_000), ("200k", 200_000), ("1M", 1_000_000)];

pub fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() as f64 / CHARS_PER_TOKEN).ceil() as usize
}

pub fn percentage_of(tokens: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    tokens as f64 * 100.0 / total as f64
}

pub fn top_heaviest(files: &[SourceFile], count: usize) -> Vec<&SourceFile> {
    let mut ranked: Vec<&SourceFile> = files.iter().collect();
    ranked.sort_by_key(|file| Reverse(file.tokens));
    ranked.truncate(count);
    ranked
}

#[cfg(test)]
mod tests {
    use super::{estimate_tokens, percentage_of, top_heaviest};
    use crate::scanner::SourceFile;

    #[test]
    fn estimates_tokens_rounding_up() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 2);
        assert_eq!(estimate_tokens(&"a".repeat(38)), 10);
    }

    #[test]
    fn computes_percentages_safely() {
        assert_eq!(percentage_of(50_000, 200_000), 25.0);
        assert_eq!(percentage_of(10, 0), 0.0);
    }

    #[test]
    fn ranks_heaviest_files_first() {
        let files: Vec<SourceFile> = [("a", 5), ("b", 50), ("c", 20), ("d", 1)]
            .into_iter()
            .map(|(path, tokens)| SourceFile {
                path: path.into(),
                tokens,
                ..Default::default()
            })
            .collect();
        let ranked: Vec<&str> = top_heaviest(&files, 3)
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(ranked, ["b", "c", "a"]);
    }
}
