Return the full record for a single item by its fully-qualified path. Requires an exact version — call `resolve_version` first if you only have a semver requirement.

`path` accepts either canonical or re-export paths (e.g. `"axum::Router"` resolves to its canonical `"axum::routing::Router"`); the `path` field on the result is always the canonical path. Returns the same `target` defaulting and fallback semantics as `search_items` / `list_module`.

The result includes:
  - `kind`, `name`, `path`, `id`
  - `inner`: structured rustdoc info (signature, generics, where-clauses, fields/variants/function decl, etc.); shape varies by `kind`
  - `deprecation`, `span`, `attrs`

`verbosity` controls how much detail to return (default `"full"`):
  - `"signature"`: signature only (the structured `inner`). Cheap.
  - `"full"`: signature + raw `docs` + `examples` (Rust fenced code blocks extracted from the doc string). Blocks tagged with non-Rust languages are skipped; rustdoc attributes like `ignore`, `no_run`, `compile_fail`, `editionXXXX` are treated as Rust. Hidden doctest lines (starting with `#`) are kept verbatim — strip or substitute as needed.

Follow-ups: when the item is a struct/enum/union, call `list_methods` on the same path to see its methods. When it's a module, call `list_module` to enumerate its children. When it's a trait, `list_methods` on a concrete type that implements it shows which methods are inherited; `get_item` on the trait itself shows the trait's required/default methods inside `inner`.
