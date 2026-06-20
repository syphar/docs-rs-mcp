use std::collections::{BTreeSet, HashSet};

use crate::{client::get_source::fetch_cargo_manifest, context::Context, errors::Error};
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Feature {
    pub(crate) name: String,
    /// The other features/deps this feature pulls in (the values from the
    /// Cargo.toml feature list, verbatim — e.g. `"dep:tokio"`, `"foo/std"`,
    /// `"some_other_feature"`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) enables: Vec<String>,
    /// True if this feature is part of the crate's `default` feature set.
    pub(crate) default: bool,
}

pub(crate) async fn inspect_feature_flags(
    context: &Context,
    krate: &str,
    version: &semver::Version,
) -> Result<Vec<Feature>, Error> {
    let manifest = fetch_cargo_manifest(context, krate, version).await?;
    let mut features = manifest.features.clone().unwrap_or_default();
    let optional_dependencies = optional_dependency_features(&manifest);
    let explicitly_suppressed: HashSet<_> = features
        .values()
        .flatten()
        .filter_map(|entry| entry.strip_prefix("dep:"))
        .map(str::to_string)
        .collect();
    for dependency in optional_dependencies {
        if !explicitly_suppressed.contains(&dependency) {
            features
                .entry(dependency.clone())
                .or_insert_with(|| vec![format!("dep:{dependency}")]);
        }
    }
    if features.is_empty() {
        return Ok(Vec::new());
    }

    let defaults: Vec<String> = features.remove("default").unwrap_or_default();

    let mut out: Vec<Feature> = features
        .into_iter()
        .map(|(name, enables)| Feature {
            default: defaults.iter().any(|d| d == &name),
            name,
            enables,
        })
        .collect();
    // Re-insert the `default` feature row itself so the caller sees what it
    // expands to (we removed it above only so it wasn't treated as a regular
    // feature when checking which features are in defaults).
    if !defaults.is_empty() {
        out.push(Feature {
            name: "default".to_string(),
            enables: defaults,
            default: false,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn optional_dependency_features(manifest: &cargo_manifest::Manifest) -> BTreeSet<String> {
    manifest
        .dependencies
        .iter()
        .flat_map(|dependencies| dependencies.iter())
        .filter_map(|(name, dependency)| match dependency {
            cargo_manifest::Dependency::Detailed(detail) if detail.optional.unwrap_or(false) => {
                Some(name.clone())
            }
            cargo_manifest::Dependency::Inherited(inherited)
                if inherited.optional.unwrap_or(false) =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[tokio::test]
    async fn test_axum_features() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        let features = inspect_feature_flags(env.context(), "axum", &version).await?;

        let names: Vec<&str> = features.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"default"));
        assert!(names.contains(&"json"));
        assert!(names.contains(&"ws"));

        // At least some features are in default (e.g. json, tokio, http1).
        assert!(features.iter().any(|f| f.default));

        Ok(())
    }
}
