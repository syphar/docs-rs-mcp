Search one exact crate version's rustdoc items by name, path, or doc alias. Use
to discover structs, traits, functions, modules, or re-exports when the exact path is
unknown; use `get_item`, `list_methods`, or `list_module` to inspect a match.

Returns target-resolution metadata, `{items: [{id, name, path, kind, aliases,
reexport?}]}`, and `unexpanded_external_globs`. The same item can appear at canonical
and re-export paths; deduplicate by `id` only when that is desired. External glob
re-exports are reported rather than expanded. Returns rustdoc-loading errors and
requires an exact version. Set `target` only when the project target differs from the
host. Items marked `#[doc(hidden)]` can appear and are not stable public API.
