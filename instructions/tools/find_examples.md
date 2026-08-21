Find runnable Rust examples published under a crate release's `examples/` directory.
Prefer this for working examples; use `get_item` when doctests for a specific API item
are more relevant.

Returns `{examples: [{path, name, required_features, content?}]}`. Source is included
only when `include_content=true`. An empty `examples` array means the crate published
no example files. Requires an exact version and returns an error when the release
archive cannot be fetched or parsed.
