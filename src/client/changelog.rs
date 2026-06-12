use crate::{client::get_source::fetch_source, context::Config};
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Changelog {
    /// Name of the file the changelog was read from (e.g. `"CHANGELOG.md"`).
    pub(crate) source_file: String,
    /// Raw file content. If the changelog is huge, the caller can scope by
    /// passing a `version` filter to extract just one release.
    pub(crate) content: String,
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
    config: &Config,
    krate: &str,
    version: &semver::Version,
    section_version: Option<&str>,
) -> Result<Option<Changelog>> {
    let Some(source_dir) = fetch_source(config, krate, version).await? else {
        return Ok(None);
    };

    let mut changelog = None;
    for candidate in CANDIDATES {
        let candidate = source_dir.join(candidate);
        if candidate.exists() {
            changelog = Some(candidate);
            break;
        }
    }

    let Some(changelog) = changelog else {
        return Ok(None);
    };

    let bytes = tokio::fs::read(&changelog).await?;

    let text = String::from_utf8_lossy(&bytes).into_owned();
    let content = match section_version {
        Some(v) => extract_version_section(&text, v).unwrap_or(text),
        None => text,
    };

    Ok(Some(Changelog {
        source_file: changelog.display().to_string(),
        content,
    }))
}

/// Best-effort section extractor. Looks for a markdown heading containing the
/// requested version string and captures lines up to the next heading at the
/// same or higher level. Heuristic — changelog formats vary. Returns `None`
/// if no matching heading is found.
fn extract_version_section(text: &str, version: &str) -> Option<String> {
    let lines = text.lines().peekable();
    let mut start_level: Option<usize> = None;
    let mut captured = String::new();
    for line in lines {
        let level = heading_level(line);
        if start_level.is_none() {
            // Look for a heading line that mentions the version.
            if level.is_some() && line.contains(version) {
                start_level = level;
                captured.push_str(line);
                captured.push('\n');
            }
        } else {
            // Capturing — stop at the next heading at the same-or-higher level.
            #[allow(clippy::unnecessary_unwrap)]
            if let Some(lvl) = level
                && lvl <= start_level.unwrap()
            {
                break;
            }
            captured.push_str(line);
            captured.push('\n');
        }
    }
    let trimmed = captured.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let hashes = trimmed.bytes().take_while(|b| *b == b'#').count();
    (hashes > 0 && hashes <= 6 && trimmed.as_bytes().get(hashes) == Some(&b' ')).then_some(hashes)
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

        let cl = changelog(env.config(), "axum", &version, None)
            .await?
            .expect("changelog present");
        assert!(cl.source_file.ends_with("/CHANGELOG.md"));
        assert!(!cl.content.is_empty());
        Ok(())
    }

    #[test]
    fn test_extract_version_section() {
        let text = "\
# Changelog

## 1.0.0 — 2026-01-01

- Big release.
- Another change.

## 0.9.0 — 2025-12-01

- Older stuff.
";
        let section = extract_version_section(text, "1.0.0").unwrap();
        assert!(section.contains("Big release"));
        assert!(!section.contains("Older stuff"));
    }
}
