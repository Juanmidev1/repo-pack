use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::handle;

fn request(root: &Path, message: Value) -> Value {
    handle(&message.to_string(), root).expect("requests with an id get a response")
}

fn call(root: &Path, name: &str, arguments: Value) -> Value {
    let message = json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": { "name": name, "arguments": arguments } });
    request(root, message)["result"].clone()
}

#[test]
fn answers_initialize_and_lists_tools() {
    let root = Path::new(".");
    let init = request(
        root,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-03-26" } }),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
    let tools = request(
        root,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    );
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["repo_map", "read_files", "pack_impact"]);
}

#[test]
fn ignores_notifications_and_reports_protocol_errors() {
    let root = Path::new(".");
    let notification = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    assert!(handle(&notification.to_string(), root).is_none());
    assert_eq!(handle("{not json", root).unwrap()["error"]["code"], -32700);
    let unknown = request(root, json!({ "jsonrpc": "2.0", "id": 3, "method": "nope" }));
    assert_eq!(unknown["error"]["code"], -32601);
}

#[test]
fn tools_map_and_read_files() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("lib.rs"),
        "pub fn add(a: i32) -> i32 {\n    a + 1\n}\n",
    )
    .unwrap();
    fs::write(root.path().join("notes.md"), "# Notes").unwrap();
    let map = call(root.path(), "repo_map", json!({}));
    assert!(
        map["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("lib.rs - ")
    );
    let read = call(
        root.path(),
        "read_files",
        json!({ "files": ["lib.rs"], "skeleton": true }),
    );
    let text = read["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("mode=\"skeleton\">\npub fn add(a: i32) -> i32 { ... }"));
    assert!(!text.contains("notes.md\""));
    let missing = call(root.path(), "read_files", json!({ "files": ["nope.rs"] }));
    assert_eq!(missing["isError"], true);
}
