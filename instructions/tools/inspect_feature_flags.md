Answer *"how do I enable feature X?"*, *"what features does this crate have?"*, *"does this crate need `tokio/macros`?"*. Reads the `[features]` section from the crate's `Cargo.toml`.

Each row is `{name, enables, default}`:
  - `enables`: the verbatim entries Cargo will activate (other features, `dep:foo` syntax for optional deps, `foo/bar` for transitive feature activation).
  - `default`: true when this feature is in the crate's `default` feature set.

Requires an exact `version` (the published Cargo.toml on crates.io for that release).
