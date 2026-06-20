use crate::{client::get_source::fetch_source, context::Context, errors::Error};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use tokio::task::spawn_blocking;
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
pub(crate) struct SourceMatch {
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) snippet: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct SourceFile {
    pub(crate) path: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) content: String,
    pub(crate) content_truncated: bool,
}

pub(crate) async fn search_source(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    query: &str,
    path_glob: &str,
    limit: usize,
    context_lines: usize,
) -> Result<Vec<SourceMatch>, Error> {
    let root = fetch_source(context, krate, version).await?;
    let query = query.to_lowercase();
    let path_glob = path_glob.to_string();

    spawn_blocking(move || -> anyhow::Result<Vec<SourceMatch>> {
        let mut matches = Vec::new();
        for entry in WalkDir::new(&root).follow_links(false) {
            let entry = entry.map_err(anyhow::Error::from)?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(&root)
                .map_err(anyhow::Error::from)?;
            let relative_string = relative.to_string_lossy().replace('\\', "/");
            if !glob_matches(&path_glob, &relative_string) {
                continue;
            }
            let Ok(content) = fs::read_to_string(entry.path()) else {
                continue;
            };
            let lines: Vec<_> = content.lines().collect();
            for (index, line) in lines.iter().enumerate() {
                if !line.to_lowercase().contains(&query) {
                    continue;
                }
                let start = index.saturating_sub(context_lines);
                let end = (index + context_lines + 1).min(lines.len());
                matches.push(SourceMatch {
                    path: relative_string.clone(),
                    line: index + 1,
                    snippet: lines[start..end].join("\n"),
                });
                if matches.len() == limit {
                    return Ok(matches);
                }
            }
        }
        Ok(matches)
    })
    .await
    .map_err(anyhow::Error::from)?
    .map_err(Error::from)
}

pub(crate) async fn read_source_file(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    path: &str,
    start_line: usize,
    end_line: Option<usize>,
    max_chars: usize,
) -> Result<SourceFile, Error> {
    let root = fetch_source(context, krate, version).await?;
    let relative = safe_relative_path(path)?;
    let requested_path = root.join(&relative);
    let canonical_root = tokio::fs::canonicalize(&root).await?;
    let canonical_path = tokio::fs::canonicalize(&requested_path).await?;
    if !canonical_path.starts_with(&canonical_root) || !canonical_path.is_file() {
        return Err(Error::ResourceNotFound(format!("source file {path}")));
    }

    let content = tokio::fs::read_to_string(canonical_path).await?;
    let lines: Vec<_> = content.lines().collect();
    let start = start_line.max(1);
    let requested_end = end_line.unwrap_or(start.saturating_add(199));
    let end = requested_end.min(lines.len()).max(start.saturating_sub(1));
    let selected = if start > lines.len() {
        String::new()
    } else {
        lines[start - 1..end].join("\n")
    };
    let content_truncated = selected.chars().count() > max_chars;

    Ok(SourceFile {
        path: relative.to_string_lossy().replace('\\', "/"),
        start_line: start,
        end_line: end,
        content: selected.chars().take(max_chars).collect(),
        content_truncated,
    })
}

fn safe_relative_path(path: &str) -> Result<PathBuf, Error> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::ResourceNotFound(format!("source file {path:?}")));
    }
    Ok(path.to_path_buf())
}

fn glob_matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let mut memo = vec![vec![None; text.len() + 1]; pattern.len() + 1];

    fn matches(
        pattern: &[char],
        text: &[char],
        pi: usize,
        ti: usize,
        memo: &mut [Vec<Option<bool>>],
    ) -> bool {
        if let Some(value) = memo[pi][ti] {
            return value;
        }
        let value = if pi == pattern.len() {
            ti == text.len()
        } else if pattern[pi] == '*' {
            matches(pattern, text, pi + 1, ti, memo)
                || (ti < text.len() && matches(pattern, text, pi, ti + 1, memo))
        } else if ti < text.len() && (pattern[pi] == '?' || pattern[pi] == text[ti]) {
            matches(pattern, text, pi + 1, ti + 1, memo)
        } else {
            false
        };
        memo[pi][ti] = Some(value);
        value
    }

    matches(&pattern, &text, 0, 0, &mut memo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_supports_recursive_rust_patterns() {
        assert!(glob_matches("**/*.rs", "src/client/source.rs"));
        assert!(glob_matches("*.rs", "lib.rs"));
        assert!(glob_matches("*.rs", "src/lib.rs"));
        assert!(!glob_matches("**/*.rs", "README.md"));
    }

    #[test]
    fn rejects_parent_paths() {
        assert!(safe_relative_path("../Cargo.toml").is_err());
        assert!(safe_relative_path("src/lib.rs").is_ok());
    }
}
