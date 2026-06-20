Search the published crate source archive for text. Returns relative file paths, one-based line numbers, and bounded context snippets.

Use this when rustdoc describes API shape but not implementation behavior, error origins, internal feature gates, tests, or private helpers. `path_glob` defaults to `*.rs` (`*` also spans directories); `limit` is capped at 100 and `context_lines` at 5. Follow up with `read_source_file`.
