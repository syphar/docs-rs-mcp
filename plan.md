 Plan: MCP tools for Rust crate docs (v1)

 Context

 docs-rs-mcp already fetches and caches rustdoc JSON from docs.rs and parses it into
 rustdoc_types::Crate via get_docs() in src/rustdoc_json.rs. The server scaffold
 (src/counter.rs — to be replaced) shows the rmcp #[tool_router] pattern.

 What's missing is the actual surface area: MCP tools that let Claude turn a parsed
 rustdoc Crate into the answers it needs while writing Rust code. v1 should give
 Claude enough to: find items, read them, walk modules, pin a version, and answer
 "what can I do with type T / who implements trait U".

 v1 tools

 All tools take (krate: String, version: Option<String>); version defaults to
 "latest" and is resolved through get_docs. Results return the public path a
 user would type in use — re-exports are resolved to that path, not internal ones.

 1. resolve_version(krate, req?) — semver req → concrete version + list of
 available versions. Call this before others when the user gives a range. Backed
 by the docs.rs API (https://docs.rs/crate/{krate}/{req} redirects, or the
 releases endpoint — pick one in implementation).
 2. search_items(krate, query, kind?, limit?) — substring/fuzzy search over
 item names in Crate.index. Returns [{path, kind, summary}]. kind filters
 to function|struct|enum|trait|macro|module|.... The most-used tool.
 3. list_module(krate, path?) — children of a module (or crate root if path
 is None). Returns one row per child: {name, kind, summary, deprecated}.
 4. get_item(krate, path, verbosity?) — full record for one item: signature,
 generics, where-clauses, doc string, deprecation, stability, source span, and
 (for items with code-block examples in docs) extracted rust fenced blocks.
 verbosity = "signature" | "full" — collapses the would-be get_signature and
 get_examples into a flag instead of new tools.
 5. list_methods(krate, type_path) — inherent methods + methods inherited from
 trait impls on the type. Walks Item::Impl entries whose for_ resolves to
 type_path. Returns [{name, signature, via_trait?, summary}].
 6. list_impls(krate, type_path) — traits implemented by a type. Returns
 [{trait_path, generics, is_synthetic, is_blanket}].
 7. list_implementors(krate, trait_path) — types implementing a trait. Inverse
 of list_impls. Returns [{type_path, generics}].

 Critical files

 - src/rustdoc_json.rs — already has get_docs(); add nothing here unless we need
 a version-list endpoint for resolve_version (likely a new list_versions()).
 - src/main.rs — wire up the new handler instead of Counter.
 - New src/docs_tools.rs — replaces counter.rs. Holds the DocsServer
 struct with #[tool_router] impl exposing the seven tools above.
 - New src/index.rs — pure functions over rustdoc_types::Crate:
   - resolve_path(&Crate, &str) -> Option<Id> (public-path lookup, follows Use)
   - public_path_of(&Crate, Id) -> Option<String> (canonical public path)
   - search(&Crate, query, kind_filter, limit)
   - module_children(&Crate, Option<&str>)
   - methods_of(&Crate, Id), impls_of(&Crate, Id), implementors_of(&Crate, Id)
   - examples_in_doc(&str) -> Vec<String> (extract ```rust fences)

 Keeping the logic in index.rs as plain functions over &Crate makes it unit-
 testable without spinning up the MCP server. docs_tools.rs is then a thin layer
 that calls get_docs then dispatches to index.

 Re-export handling

 rustdoc_types represents re-exports as ItemEnum::Use { source, id, .. }. When
 returning a path for any item, prefer the shortest public path reachable from the
 crate root through module children and Use items (BFS from root, dedupe by Id,
 shortest wins). Compute this once per loaded crate and cache it on the
 DocsServer keyed by (krate, version).

 Schema notes

 - Use schemars::JsonSchema on argument structs (same pattern as StructRequest
 in counter.rs).
 - Keep tool return values as JSON-serialized strings via Content::text for now
 (simplest); revisit if rmcp gains better structured returns.
 - kind filter for search_items should be a string enum on the schema so the
 client surfaces the choices.

 Verification

 1. cargo build and cargo clippy clean.
 2. Unit tests in src/index.rs using the cached regex/latest.json fixture
 (already present at ~/Library/Caches/docs-rs-mcp/re/ge/regex/latest.json):
   - search_items("regex", "Regex") finds regex::Regex, regex::RegexBuilder.
   - get_item("regex", "regex::Regex::new") returns a function signature.
   - list_methods("regex", "regex::Regex") includes is_match, captures.
   - list_impls("regex", "regex::Regex") includes Clone, Debug.
   - Re-exported regex::Regex (the type actually lives in a private submodule)
 comes back as regex::Regex, not the internal path.
 3. End-to-end: run the server (cargo run) and hit it from Claude Code with a
 real prompt like "what methods does regex::Regex have" — confirm tool calls
 succeed and the answer is grounded in the JSON.

 Out of scope (defer)

 - Hoogle-style signature search.
 - Cross-version diffs / changelogs.
