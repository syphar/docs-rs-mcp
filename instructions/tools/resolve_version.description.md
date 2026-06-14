Answer questions like *"what's the latest version of X?"* / *"latest 0.8.x of axum"* / *"what 1.0-compatible version is out?"* / *"does docs.rs have version Y of X?"* / *"is X published?"*. Resolves a crate name + a semver requirement to a *concrete* published version, and tells you whether docs.rs built documentation for it.

ALWAYS call this first whenever the user gives anything other than a fully-specified `MAJOR.MINOR.PATCH` (e.g. `"1.2.3"`) — every other tool in this server requires an exact version string. In particular, `"*"`, `"0.8"`, `"^1.0"`, `"~1.2"`, `">=1.0, <2"` are NOT valid `version` arguments anywhere else; pass them as `req` here first.

The `req` argument is a semver requirement. Note: this server diverges from Cargo on one specific case — a bare *fully-qualified* `MAJOR.MINOR.PATCH` is treated as an EXACT match (matching docs.rs URL semantics), not Cargo's caret default. Everything else is parsed as a normal Cargo requirement.

  - `"*"`, `"latest"`, `"newest"` → latest published version overall (case-insensitive).
  - `"1.2.3"` or `"=1.2.3"` → exactly version 1.2.3.
  - `"1.2"` or `"^1.2"` → latest 1.x ≥ 1.2 (caret).
  - `"~1.2.3"` → latest 1.2.x.
  - `">=1.0, <2"` → latest version in that range.

Result is `{ version, doc_status }`. `doc_status = true` means docs.rs has built rustdoc JSON for that release — required for `search_items`, `list_module`, `get_item`, etc.
