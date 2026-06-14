Direct dependencies of a crate (one level, not transitive). Useful for *"what does this crate pull in?"* / *"is X already a dep of Y?"*. Reads `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, and target-gated sections from `Cargo.toml`.

Each row: `{name, kind: "normal"|"dev"|"build", req, optional, default_features, features, target?}`. `target` is set when the dep is gated under `[target.'cfg(...)'.dependencies]`. The name `dependency_tree` is aspirational — this is a flat one-level list, not a recursive tree.
