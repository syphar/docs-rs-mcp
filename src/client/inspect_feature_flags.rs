use crate::{client::get_source::fetch_cargo_manifest, context::Context, errors::Error};
use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Serialize)]
pub(crate) struct Feature {
    pub(crate) name: String,
    /// The other features/deps this feature pulls in (the values from the
    /// Cargo.toml feature list, verbatim — e.g. `"dep:tokio"`, `"foo/std"`,
    /// `"some_other_feature"`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) enables: Vec<String>,
    /// Full feature/dependency closure, excluding the feature itself.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) transitive_enables: Vec<String>,
    /// Optional dependencies activated directly or transitively.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) optional_dependencies: Vec<String>,
    /// Features that directly reference this feature.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) enabled_by: Vec<String>,
    pub(crate) enabled_by_default: bool,
    pub(crate) is_default_feature: bool,
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

    let defaults = features.get("default").cloned().unwrap_or_default();
    let default_closure = feature_closure("default", &features);
    let reverse = reverse_feature_edges(&features);
    let mut out: Vec<Feature> = features
        .iter()
        .map(|(name, enables)| {
            let closure = feature_closure(name, &features);
            Feature {
                name: name.clone(),
                enables: enables.clone(),
                optional_dependencies: closure
                    .iter()
                    .filter_map(|entry| optional_dependency_name(entry))
                    .collect(),
                transitive_enables: closure.into_iter().collect(),
                enabled_by: reverse.get(name).cloned().unwrap_or_default(),
                enabled_by_default: name == "default"
                    || defaults.iter().any(|entry| entry == name)
                    || default_closure.contains(name),
                is_default_feature: name == "default",
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn feature_closure(feature: &str, features: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
    fn visit(
        feature: &str,
        features: &BTreeMap<String, Vec<String>>,
        seen: &mut HashSet<String>,
        out: &mut BTreeSet<String>,
    ) {
        if !seen.insert(feature.to_string()) {
            return;
        }
        let Some(entries) = features.get(feature) else {
            return;
        };
        for entry in entries {
            out.insert(entry.clone());
            if features.contains_key(entry) {
                visit(entry, features, seen, out);
            }
        }
    }

    let mut seen = HashSet::new();
    let mut out = BTreeSet::new();
    visit(feature, features, &mut seen, &mut out);
    out
}

fn reverse_feature_edges(features: &BTreeMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
    let mut reverse: HashMap<String, Vec<String>> = HashMap::new();
    for (feature, entries) in features {
        for entry in entries {
            if features.contains_key(entry) {
                reverse
                    .entry(entry.clone())
                    .or_default()
                    .push(feature.clone());
            }
        }
    }
    for enabled_by in reverse.values_mut() {
        enabled_by.sort();
    }
    reverse
}

fn optional_dependency_name(entry: &str) -> Option<String> {
    if let Some(name) = entry.strip_prefix("dep:") {
        return Some(name.to_string());
    }
    let (dependency, _) = entry.split_once('/')?;
    Some(dependency.trim_end_matches('?').to_string())
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
        assert!(features.iter().any(|f| f.enabled_by_default));
        assert!(
            features
                .iter()
                .find(|f| f.name == "default")
                .is_some_and(|f| f.is_default_feature)
        );

        Ok(())
    }
}
