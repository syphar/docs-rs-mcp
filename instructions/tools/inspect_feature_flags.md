Answer *"how do I enable feature X?"*, *"what features does this crate have?"*, *"does this crate need `tokio/macros`?"*. Reads the `[features]` section from the crate's `Cargo.toml`.

Each row includes:
  - `enables`: the verbatim entries Cargo will activate (other features, `dep:foo` syntax for optional deps, `foo/bar` for transitive feature activation).
  - `transitive_enables`: the recursively expanded activation closure.
  - `optional_dependencies`: dependencies activated by the feature closure.
  - `enabled_by`: features that directly activate this feature.
  - `enabled_by_default`: true when the default feature closure activates it.
  - `is_default_feature`: true only for the special `default` row.

Requires an exact `version` (the published Cargo.toml on crates.io for that release).
