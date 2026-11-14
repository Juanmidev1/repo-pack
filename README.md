# repo-pack

A Rust CLI that packs a local repository into a single text block designed for an LLM context window.

> Status: in development. This version walks and filters files and writes them to `stdout`. XML/Markdown output, token counting and clipboard support are coming in the next PRs.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Pack the current repository
repo-pack .

# Only Rust and TOML, skip tests, ignore files larger than 100 KB
repo-pack ./my-project -e rs,toml --exclude "tests/*" --max-size 100KB

# Pipe into the clipboard
repo-pack . | wl-copy
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
