Read a bounded line range from one file in the published crate source archive. `path` must be relative to the crate root; absolute paths and traversal are rejected.

Defaults to 200 lines starting at `start_line`. Use `end_line` and `max_chars` to control response size. This reads the exact published release, not a repository's current default branch.
