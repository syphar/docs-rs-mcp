List `.rs` files under the crate's `examples/` directory. Many crates publish runnable examples here — real working code is often more useful than doctest fragments.

Each row: `{path, name, required_features, content?, content_truncated}`. Paths are relative to the published crate root. `content` is only included when `include_content=true`; use `name` to select one example and `max_chars` to bound source size.

Use this for *"show me how to use this crate"* / *"are there full working examples?"* — preferable to extracting doctests when the crate ships proper examples.
