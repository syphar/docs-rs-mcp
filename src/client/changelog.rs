use crate::{client::get_source::fetch_source, context::Context};
use anyhow::{Context as _, Result};
use serde::Serialize;
use tokio::fs;

#[derive(Debug, Serialize)]
pub(crate) struct Changelog {
    /// Name of the file the changelog was read from (e.g. `"CHANGELOG.md"`).
    pub(crate) source_file: String,
    /// Raw file content. If the changelog is huge, the caller can scope by
    /// passing a `version` filter to extract just one release.
    pub(crate) releases: Vec<Release>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Release {
    pub version: String,
    pub title: String,
    pub notes: String,
}

const CANDIDATES: &[&str] = &[
    "CHANGELOG.md",
    "CHANGELOG",
    "CHANGES.md",
    "CHANGES",
    "HISTORY.md",
    "HISTORY",
    "NEWS.md",
    "NEWS",
];

/// Find and return the crate's changelog. Tries the conventional filenames in
/// order; the first one that exists wins. When `version` is `Some`, returns
/// only the section for that version (best-effort heuristic — see code).
pub(crate) async fn changelog(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    section_version: Option<&str>,
) -> Result<Option<Changelog>> {
    let Some(source_dir) = fetch_source(context, krate, version).await? else {
        return Ok(None);
    };

    let mut filename = None;
    for candidate in CANDIDATES {
        let candidate = source_dir.join(candidate);
        if fs::try_exists(&candidate).await? {
            filename = Some(candidate);
            break;
        }
    }

    let Some(filename) = filename else {
        return Ok(None);
    };

    let bytes = fs::read(&filename).await?;
    let text = String::from_utf8_lossy(&bytes);

    let entries = parse_changelog::parse(&text).context("error parsing changelog")?;

    let filtered: Vec<_> = if let Some(section_version) = section_version {
        let Some(release) = entries.get(section_version) else {
            return Ok(None);
        };
        vec![release]
    } else {
        entries.values().collect::<Vec<_>>()
    };

    Ok(Some(Changelog {
        source_file: filename.display().to_string(),
        releases: filtered
            .into_iter()
            .map(|cl| Release {
                version: cl.version.to_string(),
                title: cl.title.to_string(),
                notes: cl.notes.to_string(),
            })
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[tokio::test]
    async fn test_axum_changelog() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        let cl = changelog(env.context(), "axum", &version, None)
            .await?
            .expect("changelog present");
        assert!(cl.source_file.ends_with("/CHANGELOG.md"));

        assert_eq!(cl.releases.len(), 90);
        assert!(
            cl.releases
                .iter()
                .all(|r| !(r.notes.is_empty() || r.version.is_empty() || r.title.is_empty()))
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_axum_changelog_single() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        let cl = changelog(env.context(), "axum", &version, Some("0.8.8"))
            .await?
            .expect("changelog present");
        assert!(cl.source_file.ends_with("/CHANGELOG.md"));

        assert_eq!(cl.releases.len(), 1);

        let r = &cl.releases[0];
        assert!(!(r.notes.is_empty() || r.version.is_empty() || r.title.is_empty()));
        Ok(())
    }

    #[tokio::test]
    async fn test_axum_changelog_unknown() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        assert!(
            changelog(env.context(), "axum", &version, Some("9.9.9"))
                .await?
                .is_none()
        );

        Ok(())
    }
}
