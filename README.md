# repo-pack

![repo-pack demo](docs/demo.gif)

A Rust CLI that packs a repository into a single text block designed for an LLM context window.

Offline (except when cloning a URL), fast (a few milliseconds on a typical repo) and shipped as a single binary. It can cut function bodies with tree-sitter, redact secrets, focus on what changed in git and run as an MCP server.

## Installation

```bash
cargo install --path .

# Minimal build without tree-sitter (no --skeleton/--focus)
cargo install --path . --no-default-features
```

The default release binary is about 8 MB because of the tree-sitter grammars; the minimal build stays under 3 MB.

Prebuilt binaries for Linux, macOS and Windows are attached to each GitHub release.

## Usage

```bash
# Pack the current repository as XML and copy it to the clipboard
repo-pack . --copy

# Only Rust and TOML, skip tests, ignore files larger than 100 KB
repo-pack ./my-project -e rs,toml --exclude "tests/*" --max-size 100KB -o context.xml

# Markdown output, refusing anything above 100k tokens
repo-pack . --format md --max-tokens 100000 -c

# Plain stdout, ready to pipe into other commands
repo-pack . | wl-copy

# Dry run: show the tree and token statistics only
repo-pack . --stats

# Signatures only, except for the files you are working on
repo-pack . --focus "src/auth/*"

# Changed files against main plus the files that import them
repo-pack . --diff main --impact

# Pack a remote repository through a temporary shallow clone
repo-pack https://github.com/user/repo --skeleton
```

## Token budget

Every run prints a summary to `stderr`, so `stdout` only ever carries the package:

```text
repo-pack summary
  Files:   8
  Lines:   759
  Tokens:  6258
  Context: 128k 4.9% | 200k 3.1% | 1M 0.6%
  Heaviest files:
    1. src/formatter.rs - 1529 tokens (24.4%)
    2. src/scanner.rs - 1089 tokens (17.4%)
    3. src/main.rs - 1041 tokens (16.6%)
```

Tokens are estimated offline as `characters / 3.8`, a ratio calibrated for source code. It is an approximation, not an exact tokenizer count. With `--max-tokens`, the run fails with exit code 1 and writes nothing when the package is above the limit.

## Output formats

**XML (default)** keeps the project map apart from the file contents, so nested code blocks never get confused. File contents are written raw, without XML escaping:

```xml
<repository name="my-project">
<directory_tree>
my-project/
└── src/
    └── main.rs
</directory_tree>
<files>
<file path="src/main.rs">
fn main() {}
</file>
</files>
</repository>
```

**Markdown (`--format md`)** prints the directory tree in a `text` block, then one `## path` heading per file with a code block tagged with the detected language. The fence grows automatically when a file already contains backticks.

## Skeleton mode

`--skeleton` parses Rust, TypeScript/TSX, JavaScript/JSX and Python with tree-sitter and replaces every function and method body with `{ ... }` (`...` in Python, keeping docstrings). Signatures, types, structs, classes and imports stay intact. Files in other languages, or that fail to parse, are packed complete.

`--focus <glob>` (repeatable) keeps the matching files complete and skeletonizes the rest. Skeleton files are marked with `mode="skeleton"` in XML and `(skeleton)` in Markdown, and the summary reports the tokens saved. The saving depends on how much of the code lives inside function bodies (about 57% on this whole repository and 68% counting only its Rust files).

## Secrets

API keys (AWS, GitHub, Slack, OpenAI/Anthropic style), private key blocks and quoted values assigned to names such as `password`, `secret` or `api_key` are replaced with `[REDACTED:<kind>]` by default. The summary lists the affected files. Use `--allow-secrets` to keep them. Detection is pattern based, so it is a safety net, not a full secret scanner.

## Git context and presets

`--diff [REF]` packs only the files changed against the merge base with `REF` (default `HEAD`), including uncommitted and untracked files. `--impact` adds the files that import a changed one, one level deep, detected from `use`/`mod`, `import`, `require()` and Python imports. Entry files (`mod.rs`, `index.ts`, `__init__.py`) are matched by their folder name. Write `--diff=REF` when the path comes right after it.

| Preset | Equivalent |
| :--- | :--- |
| `--preset review` | `--diff main --impact` |
| `--preset arch` | `--skeleton --top 10` |
| `--preset bug` | `--diff HEAD --impact`, changed files complete and the rest as skeleton |

Flags passed explicitly take precedence over the preset.

## MCP server

`repo-pack --mcp [PATH]` serves JSON-RPC 2.0 over stdio. `PATH` is the default repository for every tool. Tools:

- `repo_map`: directory tree plus the estimated tokens of each file.
- `read_files`: the given files as XML, optionally as skeleton.
- `pack_impact`: files changed against a git ref in full, plus their importers as skeleton.

Every tool accepts a `path` (local folder or git URL) and redacts secrets. To add it to Claude Code:

```bash
claude mcp add repo-pack -- repo-pack --mcp /path/to/repo
```

## Excluded by default

- Everything listed in `.gitignore` and `.ignore`, even if the folder is not a git repository.
- Hidden files and folders, `node_modules/` and `target/`.
- Binary files: anything with a NUL byte in the first 8000 bytes or that is not valid UTF-8.
- Lockfiles (`Cargo.lock`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `uv.lock`...). Include them with `--include-lockfiles`.

## Options

| Flag | Description |
| :--- | :--- |
| `-e, --ext rs,ts` | Only include these extensions |
| `--exclude <glob>` | Exclude paths matching a glob relative to the root (repeatable) |
| `--max-size <size>` | Skip files larger than the given size (`500`, `100KB`, `2MB`) |
| `--include-lockfiles` | Include lockfiles |
| `--format <xml\|md>` | Output format (default `xml`) |
| `-o, --output <file>` | Write to a file instead of `stdout` (that file is never packed itself) |
| `-c, --copy` | Copy the result to the system clipboard instead of printing it |
| `--max-tokens <n>` | Abort without writing anything above this token count |
| `--stats` | Dry run: print the tree and statistics only |
| `--top <n>` | Number of heaviest files listed in the summary (default `3`) |
| `--skeleton` | Replace function bodies with placeholders |
| `--focus <glob>` | Keep matching files complete and skeletonize the rest (repeatable) |
| `--allow-secrets` | Do not redact detected secrets |
| `--diff [ref]` | Only pack files changed against `ref` (default `HEAD`) |
| `--impact` | With `--diff`, also pack the files that import the changed ones |
| `--preset <review\|arch\|bug>` | Workflow preset |
| `--mcp` | Run as an MCP server over stdio |

## Clipboard on Linux

On X11 and Wayland the copied text belongs to the process that set it and is lost when that process exits. `repo-pack --copy` therefore leaves a small background copy of itself serving the clipboard. It exits on its own as soon as another application copies something new.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --no-default-features
```

The CI workflow runs these checks on every pull request. Pushing a `v*` tag builds the release binaries.

## License

[MIT](LICENSE)
