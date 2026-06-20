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

pub(crate) async fn source_signature(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    span: &rustdoc_types::Span,
) -> Result<Option<String>, Error> {
    let root = fetch_source(context, krate, version).await?;
    let relative = safe_relative_path(
        span.filename
            .to_str()
            .ok_or_else(|| Error::ResourceNotFound("non-UTF-8 source span path".into()))?,
    )?;
    let path = root.join(relative);
    let content = tokio::fs::read_to_string(path).await?;
    let Some(source) = slice_span(&content, span.begin, span.end) else {
        return Ok(None);
    };
    Ok(Some(declaration_prefix(&source)))
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

fn slice_span(content: &str, begin: (usize, usize), end: (usize, usize)) -> Option<String> {
    let lines: Vec<_> = content.lines().collect();
    let (begin_line, begin_column) = begin;
    let (end_line, end_column) = end;
    if begin_line == 0 || begin_column == 0 || end_line < begin_line || end_line > lines.len() {
        return None;
    }

    let mut selected = Vec::new();
    for line_number in begin_line..=end_line {
        let line = lines.get(line_number - 1)?;
        let chars: Vec<_> = line.chars().collect();
        let start = if line_number == begin_line {
            begin_column.saturating_sub(1)
        } else {
            0
        };
        let finish = if line_number == end_line {
            end_column.saturating_sub(1).min(chars.len())
        } else {
            chars.len()
        };
        if start > finish || start > chars.len() {
            return None;
        }
        selected.push(chars[start..finish].iter().collect::<String>());
    }
    Some(selected.join("\n"))
}

fn declaration_prefix(source: &str) -> String {
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut angles = 0usize;

    for (index, ch) in source.char_indices() {
        match ch {
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            '<' => angles += 1,
            '>' => angles = angles.saturating_sub(1),
            '{' if parens == 0 && brackets == 0 && angles == 0 => {
                return source[..index].trim_end().to_string();
            }
            _ => {}
        }
    }
    source.trim().to_string()
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
    use crate::{
        client::get_item::{Verbosity, get_item},
        test_utils::{docs_fixture, fixture, test_env},
    };
    use anyhow::Result;

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

    #[test]
    fn extracts_multiline_source_signature() {
        let source = "fn before() {}\n\npub async fn serve<M>(\n    listener: TcpListener,\n    make_service: M,\n) -> Serve<M>\nwhere\n    M: Service,\n{\n    Serve::new(listener, make_service)\n}\n";
        let span = slice_span(source, (3, 1), (11, 2)).unwrap();
        assert_eq!(
            declaration_prefix(&span),
            "pub async fn serve<M>(\n    listener: TcpListener,\n    make_service: M,\n) -> Serve<M>\nwhere\n    M: Service,"
        );
    }

    #[tokio::test]
    async fn extracts_signature_from_published_span() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(fixture)
            .create();
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;
        let item = get_item(&docs, &["axum", "serve", "serve"], Verbosity::Signature).unwrap();
        let span = item.span.as_ref().unwrap();

        let signature = source_signature(env.context(), "axum", &version, span)
            .await?
            .unwrap();

        assert!(
            signature.starts_with("pub fn serve<"),
            "unexpected signature: {signature}"
        );
        assert!(signature.contains("Listener"));
        assert!(!signature.contains("Serve::new"));
        Ok(())
    }
}
