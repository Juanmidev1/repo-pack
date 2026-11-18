use super::enabled::skeletonize;

#[test]
fn keeps_rust_signatures_and_types() {
    let source = "use std::fs;\n\npub struct Point { x: i32 }\n\nimpl Point {\n    pub fn norm(&self) -> i32 {\n        self.x * self.x\n    }\n}\n";
    let expected = "use std::fs;\n\npub struct Point { x: i32 }\n\nimpl Point {\n    pub fn norm(&self) -> i32 { ... }\n}\n";
    assert_eq!(skeletonize("src/point.rs", source).unwrap(), expected);
}

#[test]
fn cuts_typescript_blocks_but_keeps_expression_arrows() {
    let source = "export class Api {\n  get(id: string): Promise<User> {\n    return fetch(id);\n  }\n}\nconst double = (n: number) => n * 2;\nconst load = async () => {\n  await run();\n};\n";
    let expected = "export class Api {\n  get(id: string): Promise<User> { ... }\n}\nconst double = (n: number) => n * 2;\nconst load = async () => { ... };\n";
    assert_eq!(skeletonize("api.ts", source).unwrap(), expected);
}

#[test]
fn keeps_python_docstrings() {
    let source = "class Repo:\n    def load(self, path):\n        \"\"\"Load the repo.\"\"\"\n        return open(path).read()\n\n    def size(self):\n        return 0\n";
    let expected = "class Repo:\n    def load(self, path):\n        \"\"\"Load the repo.\"\"\"\n        ...\n\n    def size(self):\n        ...\n";
    assert_eq!(skeletonize("repo.py", source).unwrap(), expected);
}

#[test]
fn falls_back_for_unsupported_or_broken_files() {
    assert!(skeletonize("notes.md", "# Title").is_none());
    assert!(skeletonize("broken.rs", "fn main( {").is_none());
}
