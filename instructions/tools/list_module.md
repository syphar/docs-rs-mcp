List the direct children of a module in a crate. Returns one row per child with `name`, `kind`, `summary` (first paragraph of the doc comment, if any), `deprecated`, and optionally `reexport` (when the child is a `pub use` of something else).

Requires an exact version — call `resolve_version` first if you only have a semver requirement.
Defaults the `target` to the host the server was compiled for, with the same fallback to the crate's docs.rs-default target on 404 as `search_items`.

Use this to browse a crate's surface one module at a time. `path` is the fully-qualified module path including the crate name (e.g. `"axum::extract"`); omit it to list the crate root. Non-glob `pub use` re-exports appear as their own row, with `reexport.source_crate` identifying where the underlying item lives. Glob re-exports (`pub use foo::*`) of *external* crates are reported separately via `unexpanded_external_globs` — follow up by calling `list_module` (or `search_items`) on the source crate.
