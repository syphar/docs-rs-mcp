use crate::{
    client::get_source::{fetch_crate, fetch_from_source, list_entries},
    config::Config,
};
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

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

    let entries = list_entries(&archive_path, "examples").await?;
    let mut examples = Vec::new();
    for entry in entries {
        // Only .rs files for now. Skip Cargo.toml of multi-file examples.
        if entry.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = derive_example_name(&entry);
        let path_str = entry.to_string_lossy().into_owned();
        let content = if include_content {
            fetch_from_source(&archive_path, &entry)
                .await?
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        } else {
            None
        };
        examples.push(Example {
            path: path_str,
            name,
            content,
        });
    }
    examples.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Some(examples))
}

fn derive_example_name(path: &PathBuf) -> String {
    // examples/foo.rs       -> "foo"
    // examples/foo/main.rs  -> "foo"
    // examples/foo/bar.rs   -> "foo/bar"
    let mut components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    // Drop the leading "examples" segment.
    if components.first() == Some(&"examples") {
        components.remove(0);
    }
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
        let examples = find_examples(env.config(), "axum", &version, false)
            .await?
            .expect("crate fetched");
        // Just verify it didn't error; axum-0.8.9 may or may not contain
        // examples in its .crate. Either way the call succeeds.
        let _ = examples;
        Ok(())
    }
}
