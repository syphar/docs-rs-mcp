use crate::{
    client::get_source::{fetch_source, parse_cargo_manifest},
    context::Config,
};
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Example {
    /// Cargo's idea of the example's name (what you'd pass to
    /// `cargo run --example <name>`).
    pub(crate) name: String,
    /// Absolute path of the example file on disk (in the server's extracted
    /// crate cache). The AI can read this directly with its own file tool.
    pub(crate) path: String,
    /// Features that must be enabled for this example to compile (from
    /// `required-features` in `Cargo.toml`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) required_features: Vec<String>,
    /// File contents. Included only when the caller asked for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
}

/// List examples for a crate. Uses `cargo-manifest` to read both explicit
/// `[[example]]` entries from `Cargo.toml` and auto-discovered `examples/*.rs`
/// files, respecting `autoexamples = false`. Returns `None` if the crate has
/// no examples (no `[[example]]` and no `examples/` directory).
pub(crate) async fn find_examples(
    config: &Config,
    krate: &str,
    version: &semver::Version,
    include_content: bool,
) -> Result<Option<Vec<Example>>> {
    let Some(source_dir) = fetch_source(config, krate, version).await? else {
        return Ok(None);
    };

    // `from_path` calls `complete_from_path` under the hood — fills in
    // auto-discovered examples from `examples/*.rs`.
    // let manifest = cargo_manifest::Manifest::from_path(source_dir.join("Cargo.toml"))?;
    let Some(manifest) = parse_cargo_manifest(&source_dir).await? else {
        return Ok(None);
    };

    if manifest.example.is_empty() {
        return Ok(None);
    }

    let mut examples = Vec::new();
    for product in manifest.example {
        let Some(rel_path) = product.path else {
            continue;
        };
        let Some(name) = product.name else { continue };
        let abs_path = source_dir.join(&rel_path);
        let content = if include_content {
            Some(tokio::fs::read_to_string(&abs_path).await?)
        } else {
            None
        };
        examples.push(Example {
            name,
            path: abs_path.to_string_lossy().into_owned(),
            required_features: product.required_features,
            content,
        });
    }
    examples.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Some(examples))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[tokio::test]
    async fn test_axum_examples() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        // The published `axum` crate doesn't ship its examples (those live in
        // the workspace at axum/examples/). So we expect None.
        assert!(
            find_examples(env.config(), "axum", &version, false)
                .await?
                .is_none()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_itertools_examples() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 14, 0);
        let fixture = crate::test_utils::fixture("itertools-0.14.0.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/itertools/itertools-0.14.0.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        let examples = find_examples(env.config(), "itertools", &version, false)
            .await?
            .expect("should have examples");

        assert_eq!(examples.len(), 1);
        let iris = &examples[0];
        assert_eq!(iris.name, "iris");
        assert!(iris.path.ends_with("/examples/iris.rs"));
        assert!(
            std::path::Path::new(&iris.path).is_absolute(),
            "path should be absolute so the AI can read it directly"
        );
        assert!(iris.content.is_none(), "content omitted by default");

        // With include_content=true, content is populated.
        let with_content = find_examples(env.config(), "itertools", &version, true)
            .await?
            .expect("should have examples");
        assert!(with_content[0].content.is_some());

        Ok(())
    }
}
