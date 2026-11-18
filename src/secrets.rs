use std::sync::LazyLock;

use regex::Regex;

struct Rule {
    kind: &'static str,
    pattern: Regex,
}

// Credential values may not start with `[` so earlier placeholders are not redacted twice.
// Each rule keeps capture groups 1 and 3 (context around the value) when they exist
static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    [
        (
            "private-key",
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
        ),
        ("aws-access-key", r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
        (
            "github-token",
            r"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{22,})",
        ),
        ("slack-token", r"\bxox[abprs]-[A-Za-z0-9-]{10,}"),
        ("api-key", r"\bsk-(?:ant-|proj-)?[A-Za-z0-9_-]{20,}"),
        (
            "credential",
            r#"(?i)(\b\w*(?:password|passwd|secret|api_?key|access_?key|auth_?token|access_?token)\w*\s*[:=]\s*["'])([^"'\s\[][^"'\s]{7,})(["'])"#,
        ),
    ]
    .into_iter()
    .map(|(kind, pattern)| Rule {
        kind,
        pattern: Regex::new(pattern).expect("secret patterns are valid"),
    })
    .collect()
});

/// Replaces every detected secret and returns the new content with the number of redactions
pub fn redact(content: &str) -> (String, usize) {
    let mut redacted = content.to_string();
    let mut count = 0;
    for rule in RULES.iter() {
        let matches = rule.pattern.find_iter(&redacted).count();
        if matches == 0 {
            continue;
        }
        count += matches;
        let replacement = if rule.pattern.captures_len() == 4 {
            format!("${{1}}[REDACTED:{}]${{3}}", rule.kind)
        } else {
            format!("[REDACTED:{}]", rule.kind)
        };
        redacted = rule
            .pattern
            .replace_all(&redacted, replacement.as_str())
            .into_owned();
    }
    (redacted, count)
}

#[cfg(test)]
mod tests {
    use super::redact;

    // Fake secrets are split so repo-pack does not flag its own source code
    #[test]
    fn redacts_known_token_formats() {
        let input = format!(
            "aws = {}\ngh = {}\n",
            concat!("AKIA", "ABCDEFGHIJKLMNOP"),
            concat!("ghp", "_", "abcdefghijklmnopqrstuvwxyz0123456789")
        );
        let (output, count) = redact(&input);
        assert_eq!(count, 2);
        assert_eq!(
            output,
            "aws = [REDACTED:aws-access-key]\ngh = [REDACTED:github-token]\n"
        );
    }

    #[test]
    fn keeps_key_name_when_redacting_assignments() {
        let input = concat!("DB_PASS", "WORD = \"hunter2hunter2\"");
        let (output, count) = redact(input);
        assert_eq!(count, 1);
        assert_eq!(output, "DB_PASSWORD = \"[REDACTED:credential]\"");
    }

    #[test]
    fn counts_each_secret_once() {
        let input = concat!("API_KEY = \"sk-", "ant-abcdefghijklmnopqrstuvwxyz\"");
        let (output, count) = redact(input);
        assert_eq!(count, 1);
        assert_eq!(output, "API_KEY = \"[REDACTED:api-key]\"");
    }

    #[test]
    fn redacts_private_key_blocks() {
        let input = concat!(
            "-----BEGIN RSA PRIV",
            "ATE KEY-----\nabc\n-----END RSA PRIV",
            "ATE KEY-----"
        );
        assert_eq!(redact(input), ("[REDACTED:private-key]".to_string(), 1));
    }

    #[test]
    fn leaves_regular_code_untouched() {
        let input = "let password = read_password();\nlet token = \"short\";";
        assert_eq!(redact(input), (input.to_string(), 0));
    }
}
