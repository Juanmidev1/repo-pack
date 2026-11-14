# repo-pack

A Rust CLI that packs a local repository into a single text block designed for an LLM context window.

Offline, fast (a few milliseconds on a typical repo) and shipped as a single binary under 3 MB.

## Installation

```bash
cargo install --path .
```

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

## Clipboard on Linux

On X11 and Wayland the copied text belongs to the process that set it and is lost when that process exits. `repo-pack --copy` therefore leaves a small background copy of itself serving the clipboard. It exits on its own as soon as another application copies something new.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The CI workflow runs these checks on every pull request. Pushing a `v*` tag builds the release binaries.
