List direct public children of a crate module. Use to browse a crate one module at a
time or inspect re-exports; omit `path` for the crate root.

Returns target-resolution metadata, `{entries: [{name, kind, summary?, deprecated,
reexport?}]}`, and `unexpanded_external_globs`. Canonical module paths include the crate
name. External glob re-exports are reported rather than expanded; query their source
crate separately. Returns module-not-found or rustdoc-loading errors and requires an
exact version.
