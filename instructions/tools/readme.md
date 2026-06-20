Return the crate's README content from its published source archive. Uses `package.readme` from `Cargo.toml` when present, otherwise tries common root filenames such as `README.md`, `README.markdown`, `README.txt`, `README.rst`, and `README`.

Each result has `{source_file, headings, content?, content_truncated}`. Paths are relative to the published crate root. Use `headings_only=true` to inspect the outline, `heading` to select one section, and `max_chars` to bound output.

Use this for *"show me the README"*, *"how does this crate say to get started?"*, *"what setup instructions does the README give?"*, or broad usage questions where package metadata is too terse and full examples may not exist.
