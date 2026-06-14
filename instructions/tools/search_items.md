Search rustdoc items for a crate version by name or path, optionally filtering by item kind.
Requires an exact version — call `resolve_version` first if you only have a semver requirement.

Defaults to fetching docs for the *host* target — the triple this server was compiled for, which is almost always the user's own machine. So a Windows user gets Windows docs, a Linux user gets Linux docs, etc.

Override via the `target` arg when the user's project targets something different from their host (check `Cargo.toml [build] target`, `.cargo/config.toml`, or anything they've said about deployment). The common case is macOS/Windows dev → Linux server deploy: pass `target: "x86_64-unknown-linux-gnu"` then. Using the wrong target hides items gated on `#[cfg(target_os = ...)]` and can surface items that won't compile on the real target.

If docs.rs has no build for the requested `target` (most crates only opt into one target), the server falls back to the *crate's* default target — whichever target the crate author marked as default in their docs.rs metadata. For most crates that's `x86_64-unknown-linux-gnu`, but a Windows-centric crate like `windows-sys` might default to a Windows triple. The fallback is silent and assumes the crate's API is the same across targets; cfg-gated items may then be missing or extra — verify against the user's actual target if precision matters.

Each result has: `id`, `name`, `path` (import path the user writes, e.g. `axum::Router`), `kind` (`struct`, `trait`, `function`, ...), `aliases` (values declared via `#[doc(alias = "...")]`; the query also matches against these), and optionally `reexport`.

Re-exports (`pub use ...`) are first-class. The same item may appear at multiple paths: its canonical definition and every `pub use` that re-exports it. Each path is independently importable. There is no automatic dedup — if you want one entry per item, dedup on `id`.

When `reexport` is set, the match was reached via a `pub use` chain rather than the item's canonical home:
  - `reexport.source_crate`: crate the original definition lives in (omitted when same crate as the searched one).
  - `reexport.source_version`: version of that source crate, parsed from its docs.rs URL when available.

To follow a re-export to its canonical definition, call `search_items` again with `krate = reexport.source_crate` and `version = reexport.source_version` (use `resolve_version` first if the version is missing). Repeat if that result is also a re-export.

The response also includes `unexpanded_external_globs`: glob re-exports (`pub use foo::*`) that pull from external crates. The server does not expand them — for each entry, follow up with another `search_items` call against `source_crate` and `source_version`. Items found there at path `P` are also reachable in the searched crate at `<prefix>::<P>`.

Caveats:
  - Some paths surfaced here may go through private modules (importable name is the re-export, not the canonical path).
  - `#[doc(hidden)]` items may appear — not part of the stable API.

Follow-ups: for a struct/enum/trait result, call `list_methods` to see its methods or `get_item` to read its signature, docs, and examples. For a module result, call `list_module` to enumerate its children.
