use crate::{client::get_source::fetch_source, context::Context, errors::Error};
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
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub notes_truncated: bool,
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

pub(crate) struct ChangelogQuery<'a> {
    pub(crate) section_version: Option<&'a str>,
    pub(crate) from_version: Option<&'a semver::Version>,
    pub(crate) to_version: Option<&'a semver::Version>,
    pub(crate) limit: usize,
    pub(crate) summary_only: bool,
    pub(crate) max_chars: usize,
}

/// Find and return the crate's changelog. Tries the conventional filenames in
/// order; the first one that exists wins. When `version` is `Some`, returns
/// only the section for that version (best-effort heuristic — see code).
pub(crate) async fn changelog(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    query: ChangelogQuery<'_>,
) -> Result<Changelog, Error> {
    let source_dir = fetch_source(context, krate, version).await?;

    let mut filename = None;
    for candidate in CANDIDATES {
        let candidate = source_dir.join(candidate);
        if fs::try_exists(&candidate).await? {
            filename = Some(candidate);
            break;
        }
    }

    let Some(filename) = filename else {
        return Err(Error::MissingSourceFile(CANDIDATES.join(",")));
    };

    let bytes = fs::read(&filename).await?;
    let text = String::from_utf8_lossy(&bytes);

    let entries = parse_changelog::parse(&text).context("error parsing changelog")?;

    let mut filtered: Vec<_> = if let Some(section_version) = query.section_version {
        let Some(release) = entries.get(section_version) else {
            return Err(Error::ResourceNotFound(format!(
                "version {} in changelog",
                section_version
            )));
        };
        vec![release]
    } else {
        entries
            .values()
            .filter(|release| {
                let Ok(version) = semver::Version::parse(release.version) else {
                    return query.from_version.is_none() && query.to_version.is_none();
                };
                query.from_version.is_none_or(|from| &version >= from)
                    && query.to_version.is_none_or(|to| &version <= to)
            })
            .collect::<Vec<_>>()
    };
    filtered.truncate(query.limit);

    Ok(Changelog {
        source_file: filename.display().to_string(),
        releases: filtered
            .into_iter()
            .map(|cl| {
                let notes = if query.summary_only {
                    cl.notes
                        .lines()
                        .take_while(|line| !line.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    cl.notes.to_string()
                };
                let notes_truncated = notes.chars().count() > query.max_chars;
                Release {
                    version: cl.version.to_string(),
                    title: cl.title.to_string(),
                    notes: notes.chars().take(query.max_chars).collect(),
                    notes_truncated,
                }
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    fn query(section_version: Option<&str>) -> ChangelogQuery<'_> {
        ChangelogQuery {
            section_version,
            from_version: None,
            to_version: None,
            limit: usize::MAX,
            summary_only: false,
            max_chars: usize::MAX,
        }
    }

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

        let cl = changelog(env.context(), "axum", &version, query(None)).await?;
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

        let cl = changelog(env.context(), "axum", &version, query(Some("0.8.8"))).await?;
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

        assert!(matches!(
            changelog(env.context(), "axum", &version, query(Some("9.9.9")))
                .await
                .unwrap_err(),
            Error::ResourceNotFound(_)
        ));

        Ok(())
    }
}
