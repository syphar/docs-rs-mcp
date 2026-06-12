use crate::types::rustdoc_types::ItemKind;
use rmcp::schemars;
use rustdoc_types::{Id, ItemEnum};
use serde::{Deserialize, Serialize};

#[derive(
    Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename_all = "snake_case")]
pub(crate) enum Verbosity {
    /// Structured signature, generics, fields/variants only. No docs, no
    /// examples. Cheap when you just want to know the shape of an item.
    Signature,
    /// Everything: signature + the item's doc string + Rust code blocks
    /// extracted from the docs.
    #[default]
    Full,
}

#[derive(Debug, Serialize)]
pub(crate) struct ItemRecord {
    pub(crate) id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    /// Canonical path of the item — may differ from the path the caller
    /// asked for (e.g. asking for `axum::Router` returns `axum::routing::Router`).
    pub(crate) path: String,
    pub(crate) kind: ItemKind,
    /// Structured rustdoc representation — signature, generics, where-clauses,
    /// fields, variants, function decl, etc. Shape varies by `kind`; see the
    /// `rustdoc_types::ItemEnum` variants for the full schema.
    pub(crate) inner: rustdoc_types::ItemEnum,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) deprecation: Option<rustdoc_types::Deprecation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) span: Option<rustdoc_types::Span>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) attrs: Vec<rustdoc_types::Attribute>,
    /// Raw doc string from `///` comments. Only present when `verbosity = "full"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) docs: Option<String>,
    /// Rust fenced code blocks extracted from `docs`. Only present when
    /// `verbosity = "full"`. Blocks tagged with non-Rust languages
    /// (e.g. ```text```) are skipped; rustdoc attributes (`ignore`, `no_run`,
    /// `should_panic`, `compile_fail`, `editionXXXX`) are treated as Rust.
    /// Hidden doctest lines starting with `#` are kept verbatim.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) examples: Vec<String>,
}

/// Look up a single item by its fully-qualified path. Tries the canonical
/// path first; falls back to walking module children from the crate root
/// (following `pub use` chains) so re-export paths like `axum::Router`
/// resolve to `axum::routing::Router`. Returns `None` for paths that don't
/// resolve, or that resolve into another crate (whose items aren't in this
/// rustdoc JSON).
pub(crate) fn get_item(
    docs: &rustdoc_types::Crate,
    path: &[String],
    verbosity: Verbosity,
) -> Option<ItemRecord> {
    let (id, item) = resolve_item(docs, path)?;

    let canonical_path = docs
        .paths
        .get(&id)
        .map(|s| s.path.join("::"))
        .unwrap_or_else(|| path.join("::"));

    let mut record = ItemRecord {
        id,
        name: item.name.clone(),
        path: canonical_path,
        kind: item.inner.item_kind().into(),
        inner: item.inner.clone(),
        deprecation: item.deprecation.clone(),
        span: item.span.clone(),
        attrs: item.attrs.clone(),
        docs: None,
        examples: Vec::new(),
    };

    if matches!(verbosity, Verbosity::Full)
        && let Some(d) = &item.docs
    {
        record.examples = extract_rust_code_blocks(d);
        record.docs = Some(d.clone());
    }

    Some(record)
}

pub(crate) fn resolve_item<'a>(
    docs: &'a rustdoc_types::Crate,
    path: &[String],
) -> Option<(Id, &'a rustdoc_types::Item)> {
    // Fast path: direct canonical lookup.
    if let Some(id) = docs
        .paths
        .iter()
        .find_map(|(id, s)| (s.path == path).then_some(*id))
        && let Some(item) = docs.index.get(&id)
    {
        return Some((id, item));
    }
    // Slow path: walk from root, following `pub use` chains.
    walk_from_root(docs, path)
}

fn walk_from_root<'a>(
    docs: &'a rustdoc_types::Crate,
    path: &[String],
) -> Option<(Id, &'a rustdoc_types::Item)> {
    // Strip the leading crate-name segment if present.
    let crate_name = docs
        .paths
        .get(&docs.root)
        .and_then(|s| s.path.first())
        .map(String::as_str);
    let segments: &[String] = if path.first().map(String::as_str) == crate_name {
        &path[1..]
    } else {
        path
    };

    let mut current = chase_use_chain(docs, docs.root)?;
    for segment in segments {
        let ItemEnum::Module(m) = &current.1.inner else {
            return None;
        };
        let mut next = None;
        for child_id in &m.items {
            let child = docs.index.get(child_id)?;
            let name = match &child.inner {
                ItemEnum::Use(u) => Some(u.name.as_str()),
                _ => child.name.as_deref(),
            };
            if name == Some(segment.as_str()) {
                next = chase_use_chain(docs, *child_id);
                break;
            }
        }
        current = next?;
    }
    Some(current)
}

fn chase_use_chain(docs: &rustdoc_types::Crate, start: Id) -> Option<(Id, &rustdoc_types::Item)> {
    let mut id = start;
    for _ in 0..32 {
        let item = docs.index.get(&id)?;
        match &item.inner {
            ItemEnum::Use(u) => id = u.id?,
            _ => return Some((id, item)),
        }
    }
    None
}

fn extract_rust_code_blocks(docs: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = docs.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let Some(info) = trimmed.strip_prefix("```") else {
            continue;
        };
        let is_rust = is_rust_info_string(info);

        let mut block = String::new();
        for inner in lines.by_ref() {
            if inner.trim_start().starts_with("```") {
                if is_rust && !block.trim().is_empty() {
                    out.push(block);
                }
                break;
            }
            block.push_str(inner);
            block.push('\n');
        }
    }
    out
}

/// rustdoc-known per-block attributes that don't change the language.
const RUSTDOC_ATTRS: &[&str] = &[
    "ignore",
    "no_run",
    "should_panic",
    "compile_fail",
    "test_harness",
    "edition2015",
    "edition2018",
    "edition2021",
    "edition2024",
    "standalone_crate",
    "standalone",
];

fn is_rust_info_string(info: &str) -> bool {
    let info = info.trim();
    if info.is_empty() {
        return true;
    }
    let tokens: Vec<&str> = info
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.contains(&"rust") {
        return true;
    }
    tokens.iter().all(|t| RUSTDOC_ATTRS.contains(t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::docs_fixture;
    use anyhow::Result;

    #[tokio::test]
    async fn test_canonical_path() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let path = ["axum", "routing", "Router"].map(String::from);
        let item = get_item(&docs, &path, Verbosity::Signature).expect("Router exists");
        assert_eq!(item.kind, ItemKind::Struct);
        assert_eq!(item.path, "axum::routing::Router");
        assert!(item.docs.is_none(), "signature verbosity skips docs");

        Ok(())
    }

    #[tokio::test]
    async fn test_reexport_path_walks() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        // `axum::Router` is a re-export; canonical is `axum::routing::Router`.
        let path = ["axum", "Router"].map(String::from);
        let item = get_item(&docs, &path, Verbosity::Signature).expect("Router via re-export");
        assert_eq!(item.kind, ItemKind::Struct);
        assert_eq!(item.path, "axum::routing::Router");

        Ok(())
    }

    #[tokio::test]
    async fn test_full_extracts_examples() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        // Multipart's docs contain a Rust example block.
        let path = ["axum", "extract", "multipart", "Multipart"].map(String::from);
        let item = get_item(&docs, &path, Verbosity::Full).expect("Multipart exists");
        assert!(item.docs.is_some(), "full verbosity includes docs");
        assert_eq!(
            item.examples.len(),
            1,
            "Multipart's docs should contain one Rust code block"
        );
        assert!(item.examples[0].contains("Multipart"));

        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_path_returns_none() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let path = ["axum", "no_such_thing"].map(String::from);
        assert!(get_item(&docs, &path, Verbosity::Signature).is_none());

        Ok(())
    }

    #[test]
    fn test_code_block_extraction() {
        let docs = r#"
Some text.

```rust
let x = 1;
```

```text
not rust
```

```
fn implicit_rust() {}
```

```ignore
fn ignored_but_rust() {}
```
"#;
        let blocks = extract_rust_code_blocks(docs);
        assert_eq!(blocks.len(), 3);
        assert!(blocks[0].contains("let x = 1;"));
        assert!(blocks[1].contains("implicit_rust"));
        assert!(blocks[2].contains("ignored_but_rust"));
    }
}
