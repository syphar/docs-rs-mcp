use crate::{client::get_source::fetch_source, context::Context, errors::Error};
use anyhow::Result;
use serde::Serialize;
use tokio::{fs, task::spawn_blocking};

#[derive(Debug, Serialize)]
pub(crate) struct Example {
    /// Cargo's idea of the example's name (what you'd pass to
    /// `cargo run --example <name>`).
    pub(crate) name: String,
    /// Path relative to the published crate root.
    pub(crate) path: String,
    /// Features that must be enabled for this example to compile (from
    /// `required-features` in `Cargo.toml`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) required_features: Vec<String>,
    /// File contents. Included only when the caller asked for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) content_truncated: bool,
}

/// List examples for a crate. Uses `cargo-manifest` to read both explicit
/// `[[example]]` entries from `Cargo.toml` and auto-discovered `examples/*.rs`
/// files, respecting `autoexamples = false`. Returns `None` if the crate has
/// no examples (no `[[example]]` and no `examples/` directory).
pub(crate) async fn find_examples(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    include_content: bool,
    name_filter: Option<&str>,
    max_chars: usize,
) -> Result<Vec<Example>, Error> {
    let source_dir = fetch_source(context, krate, version).await?;

    // `from_path` calls `complete_from_path`, which fills in auto-discovered
    // examples from `examples/*.rs`.
    let cargo_toml = source_dir.join("Cargo.toml");
    let manifest = spawn_blocking(move || cargo_manifest::Manifest::from_path(cargo_toml))
        .await
        .map_err(anyhow::Error::from)?
        .map_err(anyhow::Error::from)?;

    if manifest.example.is_empty() {
        return Ok(vec![]);
    }

    let mut examples = Vec::new();
    for product in manifest.example {
        let Some(rel_path) = product.path else {
            continue;
        };
        let Some(name) = product.name else { continue };
        if name_filter.is_some_and(|filter| filter != name) {
            continue;
        }
        let abs_path = source_dir.join(&rel_path);
        let (content, content_truncated) = if include_content {
            let content = fs::read_to_string(&abs_path).await?;
            let truncated = content.chars().count() > max_chars;
            (Some(content.chars().take(max_chars).collect()), truncated)
        } else {
            (None, false)
        };
        examples.push(Example {
            name,
            path: rel_path,
            required_features: product.required_features,
            content,
            content_truncated,
        });
    }
    examples.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(examples)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

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
            find_examples(env.context(), "axum", &version, false, None, 20_000)
                .await?
                .is_empty()
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

        let examples =
            find_examples(env.context(), "itertools", &version, false, None, 20_000).await?;

        assert_eq!(examples.len(), 1);
        let iris = &examples[0];
        assert_eq!(iris.name, "iris");
        assert_eq!(iris.path, "examples/iris.rs");
        assert!(!Path::new(&iris.path).is_absolute());
        assert!(iris.content.is_none(), "content omitted by default");

        // With include_content=true, content is populated.
        let with_content = find_examples(
            env.context(),
            "itertools",
            &version,
            true,
            Some("iris"),
            20_000,
        )
        .await?;
        assert!(with_content[0].content.is_some());

        Ok(())
    }
}
