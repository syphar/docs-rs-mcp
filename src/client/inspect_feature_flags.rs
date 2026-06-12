use crate::{client::get_source::fetch_cargo_toml, config::Config};
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
    config: &Config,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<Vec<Feature>>> {
    let Some(cargo) = fetch_cargo_toml(config, krate, version).await? else {
        return Ok(None);
    };
    let features_table = cargo.get("features").and_then(|v| v.as_table());
    let Some(features_table) = features_table else {
        return Ok(Some(Vec::new()));
    };

    let defaults: Vec<String> = features_table
        .get("default")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut out: Vec<Feature> = features_table
        .iter()
        .map(|(name, value)| {
            let enables: Vec<String> = value
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            Feature {
                name: name.clone(),
                default: defaults.iter().any(|d| d == name),
                enables,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Some(out))
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

        let features = inspect_feature_flags(env.config(), "axum", &version)
            .await?
            .expect("features present");

        // axum has known features like "default", "json", "ws", "macros", ...
        let names: Vec<&str> = features.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"default"));
        assert!(names.contains(&"json"));
        assert!(names.contains(&"ws"));

        // At least some features are in default.
        assert!(features.iter().any(|f| f.default));

        Ok(())
    }
}
