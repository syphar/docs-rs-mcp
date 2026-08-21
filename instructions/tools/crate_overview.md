Give a compact first-pass orientation to an unfamiliar crate at one exact version.
Prefer this for broad questions such as “what does this crate provide?” before drilling
into individual API items.

Returns target-resolution metadata, package metadata, default features, unconditional
direct runtime dependencies, public root modules, and up to 4,000 README characters.
The README may be absent. Follow with `search_items`, `list_module`, `get_item`, or
`list_methods` for API details. Rustdoc-unavailable and source-fetch failures are
reported as errors.
