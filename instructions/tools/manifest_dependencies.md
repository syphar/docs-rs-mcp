Dependencies declared in a crate's manifest (one level, not transitive). Useful for *"what does this crate directly depend on?"* / *"is X declared as a dependency of Y?"*. Reads `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, and target-gated sections from `Cargo.toml`.

Each row: `{name, kind: "normal"|"dev"|"build", req, optional, default_features, features, target?}`. `target` is set when the dependency is gated under `[target.'cfg(...)'.dependencies]`.
