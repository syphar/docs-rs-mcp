Return dependencies declared directly in one exact crate release's `Cargo.toml`. Use
for manifest dependency questions—not for a resolved or transitive dependency graph.

Returns `{dependencies: [{name, package?, kind, req?, optional, default_features,
features, target?}]}` across normal, development, build, and target-gated sections.
`package` identifies the actual crate for renamed dependencies; `kind` is `normal`,
`dev`, or `build`. Returns an error when the release manifest cannot be fetched or
parsed.
