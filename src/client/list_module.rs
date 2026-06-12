use crate::{
    client::search_items::{
        Reexport, UnexpandedExternalGlob, parse_version_from_docs_rs_url, reexport_info,
        resolve_through_uses,
    },
    types::rustdoc_types::ItemKind,
};
use rustdoc_types::{Id, ItemEnum};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) kind: ItemKind,
    /// First paragraph of the item's doc comment, joined onto one line.
    /// `None` when the item has no docs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) summary: Option<String>,
    /// `true` when the item carries a `#[deprecated]` attribute.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) deprecated: bool,
    /// Set when this child is a `pub use` re-export (one row per non-glob
    /// re-export at this module level). Same shape as in `search_items`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reexport: Option<Reexport>,
}

pub(crate) struct Listing {
    pub(crate) entries: Vec<Entry>,
    /// External glob re-exports (`pub use foo::*`) at this module level.
    /// Caller (the AI) should follow up by listing the source crate.
    pub(crate) unexpanded_external_globs: Vec<UnexpandedExternalGlob>,
}

/// List direct children of a module. Returns `None` when no module exists
/// at `path` (or when `path = Some(...)` doesn't resolve to a module).
/// `path = None` lists the crate root.
pub(crate) fn list_module(
    docs: &rustdoc_types::Crate,
    path: Option<&[String]>,
) -> Option<Listing> {
    let (module_id, module_path_str) = resolve_module(docs, path)?;
    let item = docs.index.get(&module_id)?;
    let ItemEnum::Module(m) = &item.inner else {
        return None;
    };

    let mut entries: Vec<Entry> = m
        .items
        .iter()
        .filter_map(|child_id| {
            let child = docs.index.get(child_id)?;
            entry_from_child(docs, child)
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.kind.cmp(&b.kind)));

    let unexpanded_external_globs = external_globs_in(docs, &m.items, &module_path_str);

    Some(Listing {
        entries,
        unexpanded_external_globs,
    })
}

fn resolve_module(
    docs: &rustdoc_types::Crate,
    path: Option<&[String]>,
) -> Option<(Id, String)> {
    match path {
        None => {
            let summary = docs.paths.get(&docs.root)?;
            Some((docs.root, summary.path.join("::")))
        }
        Some(p) => docs.paths.iter().find_map(|(id, s)| {
            (s.path == p && matches!(s.kind, rustdoc_types::ItemKind::Module))
                .then(|| (*id, s.path.join("::")))
        }),
    }
}

fn entry_from_child(
    docs: &rustdoc_types::Crate,
    child: &rustdoc_types::Item,
) -> Option<Entry> {
    match &child.inner {
        ItemEnum::Use(u) => {
            // Globs are reported separately via `unexpanded_external_globs`
            // (external) or expanded inline by `search_items` (in-crate).
            // For a flat module listing, skip them — they don't correspond
            // to a named importable child at this level.
            if u.is_glob {
                return None;
            }
            let target_id = u.id?;
            match docs.index.get(&target_id) {
                Some(target) => {
                    let resolved = resolve_through_uses(docs, target)?;
                    Some(Entry {
                        name: u.name.clone(),
                        kind: resolved.inner.item_kind().into(),
                        summary: summary_of(resolved),
                        deprecated: resolved.deprecation.is_some(),
                        reexport: Some(reexport_info(docs, resolved)),
                    })
                }
                None => {
                    // External non-glob re-export.
                    let summary = docs.paths.get(&target_id)?;
                    let ext = docs.external_crates.get(&summary.crate_id);
                    Some(Entry {
                        name: u.name.clone(),
                        kind: summary.kind.into(),
                        summary: None, // no Item available for cross-crate
                        deprecated: false,
                        reexport: Some(Reexport {
                            source_crate: ext.map(|e| e.name.clone()),
                            source_version: ext
                                .and_then(|e| e.html_root_url.as_deref())
                                .and_then(parse_version_from_docs_rs_url),
                        }),
                    })
                }
            }
        }
        _ => Some(Entry {
            name: child.name.clone()?,
            kind: child.inner.item_kind().into(),
            summary: summary_of(child),
            deprecated: child.deprecation.is_some(),
            reexport: None,
        }),
    }
}

fn summary_of(item: &rustdoc_types::Item) -> Option<String> {
    let docs = item.docs.as_deref()?.trim();
    if docs.is_empty() {
        return None;
    }
    // Take up to (but not including) the first blank line — rustdoc's idea
    // of an item's "summary".
    let summary: String = docs
        .lines()
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    (!summary.is_empty()).then_some(summary)
}

fn external_globs_in(
    docs: &rustdoc_types::Crate,
    module_items: &[Id],
    prefix: &str,
) -> Vec<UnexpandedExternalGlob> {
    module_items
        .iter()
        .filter_map(|child_id| {
            let item = docs.index.get(child_id)?;
            let ItemEnum::Use(u) = &item.inner else {
                return None;
            };
            if !u.is_glob {
                return None;
            }
            let target_id = u.id?;
            if docs.index.contains_key(&target_id) {
                return None;
            }
            let summary = docs.paths.get(&target_id)?;
            let ext = docs.external_crates.get(&summary.crate_id)?;
            Some(UnexpandedExternalGlob {
                prefix: prefix.to_string(),
                source_crate: ext.name.clone(),
                source_version: ext
                    .html_root_url
                    .as_deref()
                    .and_then(parse_version_from_docs_rs_url),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::docs_fixture;
    use anyhow::Result;

    #[tokio::test]
    async fn test_list_crate_root() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let listing = list_module(&docs, None).expect("axum has a root module");

        let actual: Vec<(String, ItemKind)> = listing
            .entries
            .iter()
            .map(|e| (e.name.clone(), e.kind))
            .collect();

        pretty_assertions::assert_eq!(
            actual,
            vec![
                ("BoxError".to_string(), ItemKind::TypeAlias),
                ("Error".to_string(), ItemKind::Struct),
                ("Extension".to_string(), ItemKind::Struct),
                ("Form".to_string(), ItemKind::Struct),
                ("Json".to_string(), ItemKind::Struct),
                ("RequestExt".to_string(), ItemKind::Trait),
                ("RequestPartsExt".to_string(), ItemKind::Trait),
                ("Router".to_string(), ItemKind::Struct),
                ("ServiceExt".to_string(), ItemKind::Trait),
                ("body".to_string(), ItemKind::Module),
                ("debug_handler".to_string(), ItemKind::ProcAttribute),
                ("debug_middleware".to_string(), ItemKind::ProcAttribute),
                ("error_handling".to_string(), ItemKind::Module),
                ("extract".to_string(), ItemKind::Module),
                ("handler".to_string(), ItemKind::Module),
                ("http".to_string(), ItemKind::Module),
                ("middleware".to_string(), ItemKind::Module),
                ("response".to_string(), ItemKind::Module),
                ("routing".to_string(), ItemKind::Module),
                // `serve` is both a module and a free function at the crate root.
                ("serve".to_string(), ItemKind::Module),
                ("serve".to_string(), ItemKind::Function),
                ("test_helpers".to_string(), ItemKind::Module),
            ]
        );

        // Spot-check reexport attribution against the same listing.
        let by_name = |name: &str, kind: ItemKind| {
            listing
                .entries
                .iter()
                .find(|e| e.name == name && e.kind == kind)
                .unwrap_or_else(|| panic!("missing entry {name} ({kind:?})"))
        };

        // Internal re-export: axum::routing::Router → axum::Router.
        let router = by_name("Router", ItemKind::Struct);
        let rx = router.reexport.as_ref().expect("Router is a re-export");
        assert!(rx.source_crate.is_none(), "internal: source_crate is None");

        // External re-export from axum_core.
        let request_ext = by_name("RequestExt", ItemKind::Trait);
        let rx = request_ext
            .reexport
            .as_ref()
            .expect("RequestExt is a re-export");
        assert_eq!(rx.source_crate.as_deref(), Some("axum_core"));

        // Direct child module — defined here, not re-exported.
        let extract = by_name("extract", ItemKind::Module);
        assert!(extract.reexport.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_module_returns_none() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let path = vec!["axum".to_string(), "no_such_module".to_string()];
        assert!(list_module(&docs, Some(&path)).is_none());

        Ok(())
    }
}
