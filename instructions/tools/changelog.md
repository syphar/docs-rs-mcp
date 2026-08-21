Return a crate release's changelog from its published source archive. Use for questions
about changes, breaking changes, or whether a changelog exists—not for public API
signatures.

Returns `{source_file, releases: [{version, title, notes}]}`. With
`section_version`, `releases` contains only the matching parsed section; otherwise it
contains all parsed releases. Returns a resource-not-found error when no common
changelog filename or requested version section exists. Requires an exact crate
version.
