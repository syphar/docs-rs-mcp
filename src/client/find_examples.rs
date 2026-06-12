use crate::{
    client::get_source::{extract_source, fetch_crate},
    config::Config,
};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub(crate) struct Example {
    /// Path of the example file within the crate's source tree
    /// (e.g. `"examples/hello.rs"`).
    pub(crate) path: String,
    /// Inferred example name — typically the file stem, or the directory
    /// name for multi-file examples.
    pub(crate) name: String,
    /// File contents. Included only when the caller asked for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
}

/// List `.rs` files under `examples/` in the crate's source tree. When
/// `include_content` is true, also reads each file and includes its source.
pub(crate) async fn find_examples(
    config: &Config,
    krate: &str,
    version: &semver::Version,
    include_content: bool,
) -> Result<Option<Vec<Example>>> {
    let Some(archive_path) = fetch_crate(config, krate, version).await? else {
        return Ok(None);
    };

    let version = version.to_string();
    let source_dir = extract_source(&archive_path, krate, &version).await?;
    let examples_dir = source_dir.join("examples");
    if !examples_dir.exists() {
        return Ok(None);
    }

    let mut examples = Vec::new();

    for entry in walkdir::WalkDir::new(&examples_dir) {
        let entry = entry?;
        if entry.path().extension().is_none_or(|e| e != "rs") {
            continue;
        }

        let name = derive_example_name(entry.path().strip_prefix(&examples_dir)?);
        let content = if include_content {
            let content = std::fs::read(entry.path())?;
            Some(String::from_utf8_lossy(&content).into_owned())
        } else {
            None
        };
        examples.push(Example {
            path: std::path::absolute(entry.path())?.display().to_string(),
            name,
            content,
        });
    }
    examples.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Some(examples))
}

fn derive_example_name(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    // examples/foo.rs       -> "foo"
    // examples/foo/main.rs  -> "foo"
    // examples/foo/bar.rs   -> "foo/bar"
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    match components.as_slice() {
        [single] => single.trim_end_matches(".rs").to_string(),
        [dir, "main.rs"] => dir.to_string(),
        [dir, file] => format!("{dir}/{}", file.trim_end_matches(".rs")),
        _ => path.to_string_lossy().into_owned(),
    }
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
        // the workspace at axum/examples/). So we expect an empty list, but
        // the lookup itself should succeed.
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
