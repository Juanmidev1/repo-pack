use std::error::Error;
use std::io::{self, BufRead, Write};
use std::path::Path;

use clap::Parser;
use serde_json::{Value, json};

use crate::cli::{Args, OutputFormat};
use crate::{formatter, pipeline};

const PROTOCOL_VERSION: &str = "2025-06-18";

/// Newline-delimited JSON-RPC 2.0 over stdio; stdout carries protocol messages only
pub fn serve(default_root: &Path) -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle(&line, default_root) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn handle(line: &str, default_root: &Path) -> Option<Value> {
    let request: Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => return Some(error_response(Value::Null, -32700, &error.to_string())),
    };
    // Requests without an id are notifications and must never be answered
    let id = request.get("id")?.clone();
    let method = request["method"].as_str().unwrap_or_default();
    let params = &request["params"];
    let result = match method {
        "initialize" => initialize(params),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tool_definitions() }),
        "tools/call" => call_tool(params, default_root),
        _ => {
            return Some(error_response(
                id,
                -32601,
                &format!("method not found: {method}"),
            ));
        }
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn initialize(params: &Value) -> Value {
    let version = params["protocolVersion"]
        .as_str()
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "repo-pack", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn tool_definitions() -> Value {
    serde_json::from_str(include_str!("mcp/tools.json")).expect("tools.json is valid JSON")
}

fn call_tool(params: &Value, default_root: &Path) -> Value {
    let arguments = &params["arguments"];
    let outcome = match params["name"].as_str().unwrap_or_default() {
        "repo_map" => repo_map(arguments, default_root),
        "read_files" => read_files(arguments, default_root),
        "pack_impact" => pack_impact(arguments, default_root),
        other => Err(format!("unknown tool: {other}").into()),
    };
    match outcome {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
        Err(error) => {
            json!({ "content": [{ "type": "text", "text": error.to_string() }], "isError": true })
        }
    }
}

// Reusing the CLI parser keeps tool defaults and validation identical to the command line
fn build_args(
    arguments: &Value,
    default_root: &Path,
    flags: &[String],
) -> Result<Args, Box<dyn Error>> {
    let root = arguments["path"].as_str().map_or_else(
        || default_root.to_string_lossy().into_owned(),
        str::to_string,
    );
    let command_line = ["repo-pack".to_string(), root]
        .into_iter()
        .chain(flags.iter().cloned());
    Ok(Args::try_parse_from(command_line)?.with_preset())
}

fn repo_map(arguments: &Value, default_root: &Path) -> Result<String, Box<dyn Error>> {
    let package = pipeline::pack(&build_args(arguments, default_root, &[])?)?;
    let mut text = formatter::render_tree(&package.files, &package.name);
    text.push('\n');
    for file in &package.files {
        text.push_str(&format!("{} - {} tokens\n", file.path, file.tokens));
    }
    text.push_str(&format!("Total: {} tokens\n", package.tokens));
    Ok(text)
}

fn read_files(arguments: &Value, default_root: &Path) -> Result<String, Box<dyn Error>> {
    let wanted: Vec<&str> = arguments["files"]
        .as_array()
        .ok_or("`files` must be an array of paths")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let skeleton = arguments["skeleton"].as_bool().unwrap_or(false);
    let flags = if skeleton {
        vec!["--skeleton".to_string()]
    } else {
        Vec::new()
    };
    let mut package = pipeline::pack(&build_args(arguments, default_root, &flags)?)?;
    package
        .files
        .retain(|file| wanted.contains(&file.path.as_str()));
    if package.files.is_empty() {
        return Err("none of the requested files were found".into());
    }
    Ok(formatter::render(
        &package.files,
        &package.name,
        OutputFormat::Xml,
    ))
}

fn pack_impact(arguments: &Value, default_root: &Path) -> Result<String, Box<dyn Error>> {
    let base = arguments["base"].as_str().unwrap_or("HEAD");
    let flags = [
        "--preset".to_string(),
        "bug".to_string(),
        format!("--diff={base}"),
    ];
    Ok(pipeline::pack(&build_args(arguments, default_root, &flags)?)?.text)
}

#[cfg(test)]
mod tests;
