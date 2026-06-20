MCP server exposing Rust crate documentation from docs.rs rustdoc JSON.

Typical flow:
1. `resolve_version(krate, req)` to turn a semver requirement (or `*`) into a concrete version. All other tools take an exact version.
2. Discover paths with `search_items` (search by name/path/alias, filtered by kind) or `list_module` (enumerate one module's children).
3. Drill in:
   - `get_item(path)` — full record for one item: signature, docs, examples.
   - `list_methods(type_path)` — methods on a struct/enum/union.
   - `list_impls(type_path)` — traits a type implements.
   - `list_implementors(trait_path)` — types that implement a trait (single-crate only).
   - `list_module(path)` — recurse into a submodule.

Match the tool to the question:
   - any version question or non-exact version ("latest of X", "X 0.8", "X ^1.0", "does X 1.2 exist", "is docs.rs ready") → `resolve_version` (pass the requirement as `req`; use `req="*"` for latest). NEVER pass anything other than a fully-specified `MAJOR.MINOR.PATCH` as `version` to any other tool — `"*"`, `"0.8"`, `"^1.0"`, `"~1.2"`, ranges, etc. are invalid; resolve them through `resolve_version` first.
   - "what methods does X have" / "how do I use X" → `list_methods`.
   - "which traits does X implement" / "is X Send/Sync/..." → `list_impls`.
   - "what implements this trait" / "which types are X" → `list_implementors`.
   - "what's in module X" / "what does X re-export" → `list_module`.
   - "find X" / "is there a Y in this crate" → `search_items`.
   - "show me X's signature/docs/examples" → `get_item`.
   - "what features does X have" / "how do I enable X" → `inspect_feature_flags`.
   - "what is this crate" / "MSRV / license / repo" → `crate_metadata`.
   - "what dependencies does X declare" → `manifest_dependencies`.
   - "what changed in X" / "any breaking changes" → `changelog`.
   - "show me the README" / "how does this crate say to get started?" → `readme`.
   - "show me a working example" → `find_examples` first, fall back to `get_item` doctests.

Rustdoc JSON availability and version fallback:
   - `search_items`, `list_module`, `get_item`, `list_methods`, `list_impls`, and `list_implementors` require docs.rs rustdoc JSON for the exact crate version and target. Older crate releases may have no rustdoc JSON, or may have rustdoc JSON in an older format this server cannot parse.
   - If the user asks about a specific version's API, behavior, migration, or compatibility, do not silently answer from another version. Report that rustdoc JSON is unavailable or unsupported for that exact version, and optionally suggest trying a nearby/latest version as an approximation.
   - Falling back to the latest version is acceptable only for broad/current-reference questions where exact historical accuracy is not essential, such as "how do I use this crate?", "find the Router type", "what methods does the current API expose?", or "show me the README/examples". Say explicitly that the answer is based on the latest available version.
   - Do not use latest-version fallback for changelog, dependency, feature, README, example, license, MSRV, or package metadata questions about a specific release; those tools read the crate source archive and should stay tied to the requested version.
   - When a rustdoc-json tool fails for an old exact version and the user's intent is general API discovery, call `resolve_version(krate, "*")`, then retry the same rustdoc-json tool with that latest version and mention the fallback.

The `target` arg on the drill-in tools defaults to the host the server was compiled for (usually the user's machine). Override when the user's *project* targets something different (e.g. macOS dev → Linux deploy).
