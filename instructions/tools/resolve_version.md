Resolve a crate name and semver requirement to one concrete published version. Always
call this before other tools unless you already have a fully specified version such as
`1.2.3`. Use for latest-version, compatibility-range, publication, or docs.rs build
status questions.

`req` accepts Cargo semver requirements plus `latest` and `newest`; a bare complete
version is treated as exact. Returns `{version, doc_status}` where `doc_status=true`
means docs.rs built rustdoc JSON usable by rustdoc-backed tools. Returns
resource-not-found when no published release matches and an internal error on registry
or docs.rs failures.
