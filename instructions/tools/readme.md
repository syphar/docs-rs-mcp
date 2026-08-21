Return the README published with one exact crate release. Use for crate-authored setup,
usage, and introductory guidance; use `find_examples` for runnable examples and
rustdoc-backed tools for precise API details.

Returns `{source_file, content}`. Returns a resource-not-found error when the manifest
and common README filenames identify no README, or another error when the release
archive cannot be fetched or parsed.
