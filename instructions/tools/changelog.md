Return the crate's changelog content. Tries common filenames (`CHANGELOG.md`, `CHANGES.md`, `HISTORY.md`, `NEWS.md`, and their extensionless variants) and returns the first that exists.

When `section_version` is provided, returns only the heading section that mentions that version string — best-effort heuristic against markdown heading syntax. Omit it for the full changelog.

Use this for *"what changed in 1.4?"* / *"any breaking changes between 0.7 and 0.8?"* / *"is there a CHANGELOG?"*.
