Return the crate's changelog content. Tries common filenames (`CHANGELOG.md`, `CHANGES.md`, `HISTORY.md`, `NEWS.md`, and their extensionless variants) and returns the first that exists.

When `section_version` is provided, returns that parsed release. Use inclusive `from_version` / `to_version` bounds for upgrade ranges. `limit`, `summary_only`, and `max_chars` keep large changelogs bounded.

Use this for *"what changed in 1.4?"* / *"any breaking changes between 0.7 and 0.8?"* / *"is there a CHANGELOG?"*.
