use std::collections::BTreeMap;
use std::fmt::Write;

use crate::cli::OutputFormat;
use crate::scanner::SourceFile;

#[derive(Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
}

pub fn render(files: &[SourceFile], repository_name: &str, format: OutputFormat) -> String {
    let tree = render_tree(files, repository_name);
    match format {
        OutputFormat::Xml => render_xml(files, repository_name, &tree),
        OutputFormat::Md => render_markdown(files, repository_name, &tree),
    }
}

pub fn render_tree(files: &[SourceFile], repository_name: &str) -> String {
    let mut root = TreeNode::default();
    for file in files {
        let mut node = &mut root;
        for segment in file.path.split('/') {
            node = node.children.entry(segment.to_string()).or_default();
        }
    }
    let mut output = format!("{repository_name}/\n");
    write_tree_level(&root, "", &mut output);
    output
}

fn write_tree_level(node: &TreeNode, prefix: &str, output: &mut String) {
    let last_index = node.children.len().saturating_sub(1);
    for (index, (name, child)) in node.children.iter().enumerate() {
        let (branch, indent) = if index == last_index {
            ("└── ", "    ")
        } else {
            ("├── ", "│   ")
        };
        let directory_suffix = if child.children.is_empty() { "" } else { "/" };
        let _ = writeln!(output, "{prefix}{branch}{name}{directory_suffix}");
        write_tree_level(child, &format!("{prefix}{indent}"), output);
    }
}

// File contents stay unescaped on purpose: LLMs read raw code better than XML entities
fn render_xml(files: &[SourceFile], repository_name: &str, tree: &str) -> String {
    let mut output = format!(
        "<repository name=\"{}\">\n",
        escape_attribute(repository_name)
    );
    let _ = write!(
        output,
        "<directory_tree>\n{tree}</directory_tree>\n<files>\n"
    );
    for file in files {
        let path = escape_attribute(&file.path);
        let _ = writeln!(output, "<file path=\"{path}\" tokens=\"{}\">", file.tokens);
        push_with_trailing_newline(&mut output, &file.content);
        output.push_str("</file>\n");
    }
    output.push_str("</files>\n</repository>\n");
    output
}

fn render_markdown(files: &[SourceFile], repository_name: &str, tree: &str) -> String {
    let mut output = format!("# {repository_name}\n\n## Directory tree\n\n");
    push_code_block(&mut output, "text", tree);
    for file in files {
        let _ = write!(output, "\n## {}\n\n", file.path);
        push_code_block(&mut output, detect_language(&file.path), &file.content);
    }
    output
}

fn push_code_block(output: &mut String, language: &str, content: &str) {
    let fence = build_fence(content);
    let _ = writeln!(output, "{fence}{language}");
    push_with_trailing_newline(output, content);
    let _ = writeln!(output, "{fence}");
}

// The fence must be longer than any backtick run inside the content, or nested blocks break it
fn build_fence(content: &str) -> String {
    let longest_run = content
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    "`".repeat((longest_run + 1).max(3))
}

fn push_with_trailing_newline(output: &mut String, content: &str) {
    output.push_str(content);
    if !content.is_empty() && !content.ends_with('\n') {
        output.push('\n');
    }
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn detect_language(path: &str) -> &'static str {
    let extension = path.rsplit_once('.').map_or("", |(_, extension)| extension);
    match extension.to_ascii_lowercase().as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" => "cpp",
        "rb" => "ruby",
        "php" => "php",
        "sh" | "bash" => "bash",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "md" => "markdown",
        "sql" => "sql",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{render, render_tree};
    use crate::cli::OutputFormat;
    use crate::scanner::SourceFile;

    fn source(path: &str, content: &str) -> SourceFile {
        SourceFile {
            path: path.to_string(),
            content: content.to_string(),
            tokens: 7,
        }
    }

    #[test]
    fn renders_nested_tree() {
        let files = [
            source("Cargo.toml", ""),
            source("src/main.rs", ""),
            source("src/lib.rs", ""),
        ];
        let expected = "demo/\n├── Cargo.toml\n└── src/\n    ├── lib.rs\n    └── main.rs\n";
        assert_eq!(render_tree(&files, "demo"), expected);
    }

    #[test]
    fn renders_xml_with_escaped_attributes_and_raw_content() {
        let files = [source("a&b.rs", "if a < b {}")];
        let output = render(&files, "demo", OutputFormat::Xml);
        assert!(output.starts_with("<repository name=\"demo\">\n<directory_tree>\ndemo/\n"));
        assert!(output.contains("<file path=\"a&amp;b.rs\" tokens=\"7\">\nif a < b {}\n</file>\n"));
        assert!(output.ends_with("</files>\n</repository>\n"));
    }

    #[test]
    fn renders_markdown_with_language_and_safe_fence() {
        let files = [source("docs/guide.md", "```rust\nfn x() {}\n```")];
        let output = render(&files, "demo", OutputFormat::Md);
        assert!(output.contains("## docs/guide.md\n\n````markdown\n```rust\n"));
        assert!(output.ends_with("```\n````\n"));
    }
}
