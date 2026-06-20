use crate::{
    client::{
        crate_metadata::local,
        get_source::{fetch_source, parse_cargo_manifest},
    },
    context::Context,
    errors::Error,
};
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Serialize)]
pub(crate) struct Readme {
    /// Path of the README file relative to the published crate root.
    pub(crate) source_file: String,
    pub(crate) headings: Vec<String>,
    /// README contents, optionally scoped to one heading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) content_truncated: bool,
}

const DEFAULT_README_CANDIDATES: &[&str] = &["README.md", "README.txt", "README"];

async fn default_readme(source_dir: &Path) -> Result<PathBuf, Error> {
    for candidate in DEFAULT_README_CANDIDATES {
        let path = source_dir.join(candidate);
        if fs::try_exists(&path).await? {
            return Ok(path);
        }
    }

    Err(Error::MissingSourceFile(
        DEFAULT_README_CANDIDATES.join(","),
    ))
}

async fn manifest_readme(source_dir: &Path) -> Result<PathBuf, Error> {
    let manifest = parse_cargo_manifest(source_dir).await?;
    let Some(pkg) = manifest.package.as_ref() else {
        return default_readme(source_dir).await;
    };

    match local(&pkg.readme) {
        Some(cargo_manifest::StringOrBool::String(path)) => {
            let path = source_dir.join(path);
            if fs::try_exists(&path).await? {
                Ok(path)
            } else {
                Err(Error::MissingSourceFile("no readme in crate".to_string()))
            }
        }
        Some(cargo_manifest::StringOrBool::Bool(false)) => Err(Error::MissingSourceFile(
            "readme disabled in crate".to_string(),
        )),
        Some(cargo_manifest::StringOrBool::Bool(true)) | None => default_readme(source_dir).await,
    }
}

pub(crate) async fn readme(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    heading: Option<&str>,
    headings_only: bool,
    max_chars: usize,
) -> Result<Option<Readme>> {
    let source_dir = fetch_source(context, krate, version).await?;
    let readme_path = manifest_readme(&source_dir).await?;

    let bytes = fs::read(&readme_path).await?;
    let full_content = String::from_utf8_lossy(&bytes).into_owned();
    let headings = markdown_headings(&full_content);
    let selected = heading
        .map(|heading| markdown_section(&full_content, heading))
        .transpose()?
        .unwrap_or(full_content);
    let content_truncated = !headings_only && selected.chars().count() > max_chars;
    let content = (!headings_only).then(|| selected.chars().take(max_chars).collect());
    let source_file = readme_path
        .strip_prefix(&source_dir)
        .unwrap_or(&readme_path)
        .to_string_lossy()
        .into_owned();

    Ok(Some(Readme {
        source_file,
        headings,
        content,
        content_truncated,
    }))
}

fn markdown_headings(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let title = line.strip_prefix('#')?.trim_start_matches('#').trim();
            (!title.is_empty()).then(|| title.to_string())
        })
        .collect()
}

fn markdown_section(content: &str, requested: &str) -> Result<String, Error> {
    let mut selected = Vec::new();
    let mut level = None;

    for line in content.lines() {
        let trimmed = line.trim_start();
        let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
        let title = trimmed.get(hashes..).unwrap_or_default().trim();

        if level.is_none() && hashes > 0 && title.eq_ignore_ascii_case(requested) {
            level = Some(hashes);
            selected.push(line);
            continue;
        }
        if let Some(level) = level {
            if hashes > 0 && hashes <= level {
                break;
            }
            selected.push(line);
        }
    }

    if selected.is_empty() {
        Err(Error::ResourceNotFound(format!(
            "README heading {requested}"
        )))
    } else {
        Ok(selected.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[tokio::test]
    async fn test_axum_readme() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        let readme = readme(env.context(), "axum", &version, None, false, 30_000)
            .await?
            .expect("README present");

        assert_eq!(readme.source_file, "README.md");
        assert!(readme.content.as_deref().unwrap().contains("axum"));
        assert!(!readme.headings.is_empty());
        Ok(())
    }
}
