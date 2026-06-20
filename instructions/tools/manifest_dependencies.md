Dependencies declared in a crate's manifest (one level, not transitive). Useful for *"what does this crate directly depend on?"* / *"is X declared as a dependency of Y?"*. Reads `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, and target-gated sections from `Cargo.toml`.

Each row includes the manifest key and package rename, dependency kind, version requirement, optional/default-feature flags, enabled features, target gate, registry/path/git source fields, and whether the declaration is inherited from the workspace. Use `kind` and `optional_only` to filter the result.
