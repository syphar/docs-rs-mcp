# docs-rs-mcp

An MCP (Model Context Protocol) server that gives AI assistants structured,
on-demand access to Rust crate documentation from [docs.rs] and
[crates.io] — signatures, doc strings, examples, features, dependencies,
changelogs, and more.

Built so an LLM coding assistant can answer questions like *"what's the
latest version of axum?"*, *"what methods does `Router` have?"*, *"how do I
enable websockets?"* without burning tokens scraping HTML docs.

This is more or less an experiment I'm using locally to see if it gives me
real advantages with agentic coding. If it works out, we can think about
hosting an extended version of it.

## What it gives you

Couple of tools, all answering specific natural-language questions:

| Tool | Answers |
|---|---|
| `resolve_version` | *"what's the latest version of X?"*, *"does version Y exist?"* |
| `search_items` | *"find X"*, *"is there a Y in this crate?"* |
| `list_module` | *"what's in module X?"*, *"what does X re-export?"* |
| `get_item` | *"show me X's signature/docs/examples"* |
| `list_methods` | *"what methods does X have?"* |
| `list_impls` | *"which traits does X implement?"*, *"is X Send/Sync?"* |
| `list_implementors` | *"what implements this trait?"* |
| `inspect_feature_flags` | *"what features does X have?"*, *"how do I enable Y?"* |
| `crate_metadata` | *"what is this crate?"*, *"MSRV / license / repo?"* |
| `crate_overview` | *"orient me to this crate"* |
| `manifest_dependencies` | *"what dependencies does X declare?"* |
| `changelog` | *"what changed in version X?"*, *"any breaking changes?"* |
| `find_examples` | *"show me a working example"* |
| `readme` | *"how does this crate say to get started?"* |

Results are structured JSON — signatures keep their full rustdoc shape
(generics, where-clauses, fields, variants, function decl), not flattened to
strings.

## Why a Rust-specific MCP server?

- **Tool descriptions tuned for AI consumers.** Each tool is described in
  the natural-language phrasing an LLM is likely to think in, with negative
  rules ("not for X, use Y instead") to cut down on misuse.
- **Re-exports handled.** `axum::Router` resolves to its canonical
  `axum::routing::Router`; trait re-exports across crates are tracked.
- **Target awareness.** Defaults to the host's target triple, falls back to
  the crate's docs.rs-default if no build exists for the host. Cfg-gated
  items diverge cleanly.
- **Semver that matches user intuition.** `"1.2.3"` means exactly that
  version (matching docs.rs URL semantics), not Cargo's caret default.
  `"latest"` / `"newest"` are accepted as aliases for `*`.
- **All fetches are cached.** rustdoc JSON, crate sources, and version
  resolutions live under the user's cache dir; subsequent calls are local.

## known limitations

Right now, we're using just one version of rustdoc-types, which means we can
only handle the rustdoc json output from nighty versions >= `2025-08-02`.

When the docs.rs build is older, we will raise an error.

## Installation

```sh
cargo install docs-rs-mcp
```

Or build from source:

```sh
git clone https://github.com/rust-lang/docs-rs-mcp
cd docs-rs-mcp
cargo install --path .
```

The binary speaks MCP over stdio and is meant to be launched by an MCP
client — Claude Desktop, Codex, Continue, or any other tool that supports
the protocol.

## Configuration

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`
(macOS) or the equivalent on your OS:

```json
{
  "mcpServers": {
    "docs-rs": {
      "command": "docs-rs-mcp"
    }
  }
}
```

### Codex

Add to `~/.codex/config.toml`:

```toml
[mcp_servers.docs-rs]
command = "docs-rs-mcp"
```

### Other clients

Anywhere that accepts a stdio MCP server: command `docs-rs-mcp`, no
arguments. Configure logging via `RUST_LOG` if you want to see what it's
doing.

## Cache & logs

| Platform | Location |
|---|---|
| macOS | `~/Library/Caches/docs-rs-mcp/` |
| Linux | `~/.cache/docs-rs-mcp/` |
| Windows | `%LOCALAPPDATA%\docs-rs-mcp\` |

- Rustdoc JSON and crate source archives live under the cache root.
- Logs land in `_logs/` as daily-rolled JSON files, one per process
  (`docs-rs-mcp.<pid>.<date>.log`).
- Each log starts with an "instance started" event recording the PID,
  cwd, project name (if launched from a Cargo project), host target,
  and MCP server version — so a downstream analysis can correlate tool
  usage to context.

## Tips for AI consumers

The bundled tool descriptions and server-level `instructions` already cover
the typical flow:

```
resolve_version → crate_overview / search_items / list_module → get_item / list_methods / list_impls
```

A few hard rules baked into the descriptions:

- **Always go through `resolve_version` first** unless you already have a
  fully-specified `MAJOR.MINOR.PATCH`. The other tools reject `"*"`,
  `"0.8"`, `"^1.0"`, ranges, etc.
- **Re-export paths are accepted everywhere a `path` is expected** —
  `axum::Router` works the same as `axum::routing::Router`.
- **Target only matters when the project's target differs from the host.**
  macOS / Windows devs shipping to Linux: pass
  `target = "x86_64-unknown-linux-gnu"`. Otherwise leave it unset.

## Building & testing

```sh
cargo build
cargo nextest run --bin docs-rs-mcp
```

Tests use checked-in `.crate` fixtures and a mockito-backed test server, so
they run offline.

## License

Dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

[docs.rs]: https://docs.rs
[crates.io]: https://crates.io
