Return Cargo feature flags declared by one exact crate version. Use for questions about
available features, default features, optional dependencies, or how to enable a feature.

Returns `{features: [{name, enables, default}]}`. `enables` contains the manifest's
verbatim feature activations. Returns an error when the release manifest cannot be
fetched or parsed. This reports declarations, not which features the current project
actually enabled.
