Return package metadata for one exact crate version from its published `Cargo.toml`.
Use for description, repository, homepage, license, documentation URL, MSRV, edition,
authors, keywords, or categories—not for API items or version resolution.

Returns the package metadata fields as structured JSON. Call `resolve_version` first
unless the version is fully specified. Returns an error when the crate release or its
manifest cannot be fetched or parsed.
