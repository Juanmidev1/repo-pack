use std::error::Error;
use std::path::Path;

use crate::scanner::SourceFile;

/// Skeletonizes every file unless it matches a `--focus` glob; a non-empty focus implies skeleton
pub fn apply(
    files: &mut [SourceFile],
    root: &Path,
    skeleton: bool,
    focus: &[String],
) -> Result<(), Box<dyn Error>> {
    if !skeleton && focus.is_empty() {
        return Ok(());
    }
    enabled::apply(files, root, focus)
}

#[cfg(not(feature = "skeleton"))]
mod enabled {
    use super::*;

    pub fn apply(_: &mut [SourceFile], _: &Path, _: &[String]) -> Result<(), Box<dyn Error>> {
        Err("--skeleton and --focus need a build with the `skeleton` feature".into())
    }
}

#[cfg(feature = "skeleton")]
mod enabled {
    use ignore::overrides::OverrideBuilder;
    use tree_sitter::{Language, Node, Parser};

    use super::*;
    use crate::tokens::estimate_tokens;

    const FUNCTION_KINDS: &[&str] = &[
        "function_item",
        "function_declaration",
        "function_expression",
        "function",
        "generator_function_declaration",
        "generator_function",
        "method_definition",
        "arrow_function",
        "function_definition",
    ];
    // Arrow functions with expression bodies are already one-liners, so only blocks are cut
    const BODY_KINDS: &[&str] = &["block", "statement_block"];

    pub fn apply(
        files: &mut [SourceFile],
        root: &Path,
        focus: &[String],
    ) -> Result<(), Box<dyn Error>> {
        let mut builder = OverrideBuilder::new(root);
        for pattern in focus {
            builder.add(pattern)?;
        }
        let focus_matcher = builder.build()?;
        for file in files {
            if focus_matcher.matched(&file.path, false).is_whitelist() {
                continue;
            }
            if let Some(content) = skeletonize(&file.path, &file.content) {
                file.tokens = estimate_tokens(&content);
                file.content = content;
                file.skeleton = true;
            }
        }
        Ok(())
    }

    /// Returns None for unsupported languages or unparseable files, which then stay complete
    pub fn skeletonize(path: &str, content: &str) -> Option<String> {
        let extension = path.rsplit_once('.')?.1.to_ascii_lowercase();
        let language = language_for(&extension)?;
        let mut parser = Parser::new();
        parser.set_language(&language).ok()?;
        let tree = parser.parse(content, None)?;
        if tree.root_node().has_error() {
            return None;
        }
        let mut cuts = Vec::new();
        collect_cuts(tree.root_node(), content, extension == "py", &mut cuts);
        let mut output = String::with_capacity(content.len());
        let mut cursor = 0;
        for (start, end, placeholder) in cuts {
            output.push_str(&content[cursor..start]);
            output.push_str(&placeholder);
            cursor = end;
        }
        output.push_str(&content[cursor..]);
        Some(output)
    }

    fn language_for(extension: &str) -> Option<Language> {
        let language = match extension {
            "rs" => tree_sitter_rust::LANGUAGE,
            "ts" | "mts" | "cts" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            "tsx" => tree_sitter_typescript::LANGUAGE_TSX,
            "js" | "jsx" | "mjs" | "cjs" => tree_sitter_javascript::LANGUAGE,
            "py" => tree_sitter_python::LANGUAGE,
            _ => return None,
        };
        Some(language.into())
    }

    // Cut bodies are not descended into, so ranges never overlap and stay in source order
    fn collect_cuts(
        node: Node,
        source: &str,
        python: bool,
        cuts: &mut Vec<(usize, usize, String)>,
    ) {
        if FUNCTION_KINDS.contains(&node.kind())
            && let Some(body) = node.child_by_field_name("body")
            && BODY_KINDS.contains(&body.kind())
        {
            let placeholder = if python {
                python_placeholder(body, source)
            } else {
                "{ ... }".to_string()
            };
            cuts.push((body.start_byte(), body.end_byte(), placeholder));
            return;
        }
        let mut walker = node.walk();
        for child in node.children(&mut walker) {
            collect_cuts(child, source, python, cuts);
        }
    }

    // Docstrings document the contract, which is exactly what a skeleton should keep
    fn python_placeholder(body: Node, source: &str) -> String {
        let docstring = body
            .named_child(0)
            .filter(|statement| statement.kind() == "expression_statement")
            .and_then(|statement| statement.named_child(0))
            .filter(|expression| expression.kind() == "string");
        match docstring {
            Some(string) => {
                let indent = " ".repeat(body.start_position().column);
                format!("{}\n{indent}...", &source[string.byte_range()])
            }
            None => "...".to_string(),
        }
    }
}

#[cfg(all(test, feature = "skeleton"))]
mod tests;
