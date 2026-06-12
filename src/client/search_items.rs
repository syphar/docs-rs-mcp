use crate::types::rustdoc_types::ItemKind;
use rustdoc_types::{Id, ItemEnum};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
pub(crate) struct Match {
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: ItemKind,
    /// Set when this match was reached via a `pub use` re-export rather than
    /// the item's canonical path. Carries info about where the item originally
    /// lives.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reexport: Option<Reexport>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Reexport {
    /// Crate where the original item is defined. `None` if it lives in the
    /// same crate as the rustdoc JSON being searched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source_crate: Option<String>,
    /// Version of the source crate, parsed from its `html_root_url` if it
    /// looks like a docs.rs URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source_version: Option<String>,
}

pub(crate) fn search(
    docs: &rustdoc_types::Crate,
    externals: &HashMap<String, rustdoc_types::Crate>,
    query: Option<&str>,
    kind_filter: Option<ItemKind>,
    limit: Option<usize>,
) -> Vec<Match> {
    let query_lower = query.map(|q| q.to_lowercase());

    let mut matches: Vec<Match> = docs
        .index
        .values()
        .filter_map(|item| {
            // Re-exports are handled separately below.
            if matches!(item.inner, ItemEnum::Use(_)) {
                return None;
            }

            let kind: ItemKind = item.inner.item_kind().into();
            if kind_filter.is_some_and(|filter| filter != kind) {
                return None;
            }

            let path = docs
                .paths
                .get(&item.id)
                .map(|summary| summary.path.join("::"))
                .or_else(|| item.name.clone())?;
            let name = item.name.clone().unwrap_or_default();

            if !matches_query(query_lower.as_deref(), &name, &path) {
                return None;
            }

            Some(Match {
                id: item.id,
                name,
                path,
                kind,
                reexport: None,
            })
        })
        .collect();

    collect_reexports(
        docs,
        externals,
        query_lower.as_deref(),
        kind_filter,
        &mut matches,
    );

    matches.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });

    if let Some(limit) = limit {
        matches.truncate(limit);
    }

    matches
}

fn matches_query(query: Option<&str>, name: &str, path: &str) -> bool {
    match query {
        Some(q) => format!("{name} {path}").to_lowercase().contains(q),
        None => true,
    }
}

fn collect_reexports(
    docs: &rustdoc_types::Crate,
    externals: &HashMap<String, rustdoc_types::Crate>,
    query: Option<&str>,
    kind_filter: Option<ItemKind>,
    out: &mut Vec<Match>,
) {
    // Canonical path for every in-crate module, used as the prefix for paths
    // we synthesize for re-exports.
    let module_paths: HashMap<Id, String> = docs
        .index
        .iter()
        .filter(|(_, item)| matches!(item.inner, ItemEnum::Module(_)))
        .filter_map(|(id, _)| docs.paths.get(id).map(|s| (*id, s.path.join("::"))))
        .collect();

    // Parent module for each child Id, so we can find the path a `Use` sits in.
    let mut parent_of: HashMap<Id, Id> = HashMap::new();
    for item in docs.index.values() {
        if let ItemEnum::Module(m) = &item.inner {
            for child in &m.items {
                parent_of.insert(*child, item.id);
            }
        }
    }

    let use_ids: Vec<Id> = docs
        .index
        .iter()
        .filter(|(_, item)| matches!(item.inner, ItemEnum::Use(_)))
        .map(|(id, _)| *id)
        .collect();

    for use_id in use_ids {
        let parent_path = parent_of
            .get(&use_id)
            .and_then(|p| module_paths.get(p))
            .cloned()
            .unwrap_or_default();
        // Per-top-level visited set: cycle protection without preventing a
        // given Use from being emitted under multiple distinct paths.
        let mut visited = HashSet::new();
        expand_use(
            docs,
            externals,
            use_id,
            &parent_path,
            &mut visited,
            out,
            query,
            kind_filter,
        );
    }
}

fn expand_use(
    docs: &rustdoc_types::Crate,
    externals: &HashMap<String, rustdoc_types::Crate>,
    use_id: Id,
    prefix: &str,
    visited: &mut HashSet<Id>,
    out: &mut Vec<Match>,
    query: Option<&str>,
    kind_filter: Option<ItemKind>,
) {
    if !visited.insert(use_id) {
        return;
    }

    let Some(item) = docs.index.get(&use_id) else {
        return;
    };
    let ItemEnum::Use(u) = &item.inner else {
        return;
    };
    let Some(target_id) = u.id else {
        // rustdoc couldn't resolve the re-export at all.
        return;
    };
    let Some(target) = docs.index.get(&target_id) else {
        // Target lives in an external crate.
        if !u.is_glob {
            emit_external_reexport(docs, target_id, &u.name, prefix, out, query, kind_filter);
            return;
        }
        // External glob: try to expand using a fetched external crate.
        expand_external_glob(
            docs,
            externals,
            target_id,
            prefix,
            out,
            query,
            kind_filter,
        );
        return;
    };

    if !u.is_glob {
        emit_reexport(docs, target, &u.name, prefix, out, query, kind_filter);
        // Re-export chains: `pub use a::b;` where `b` is itself a `use`.
        if matches!(target.inner, ItemEnum::Use(_)) {
            let new_prefix = join_path(prefix, &u.name);
            expand_use(
                docs,
                externals,
                target_id,
                &new_prefix,
                visited,
                out,
                query,
                kind_filter,
            );
        }
        return;
    }

    // Glob in-index: target must be a module. Enumerate its children at `prefix`.
    expand_module(
        docs,
        externals,
        target_id,
        prefix,
        visited,
        out,
        query,
        kind_filter,
    );
}

fn expand_module(
    docs: &rustdoc_types::Crate,
    externals: &HashMap<String, rustdoc_types::Crate>,
    module_id: Id,
    prefix: &str,
    visited: &mut HashSet<Id>,
    out: &mut Vec<Match>,
    query: Option<&str>,
    kind_filter: Option<ItemKind>,
) {
    let Some(item) = docs.index.get(&module_id) else {
        return;
    };
    let ItemEnum::Module(m) = &item.inner else {
        return;
    };
    for child_id in &m.items {
        let Some(child) = docs.index.get(child_id) else {
            continue;
        };
        if matches!(child.inner, ItemEnum::Use(_)) {
            expand_use(
                docs,
                externals,
                *child_id,
                prefix,
                visited,
                out,
                query,
                kind_filter,
            );
        } else {
            let Some(name) = child.name.as_deref() else {
                continue;
            };
            emit_reexport(docs, child, name, prefix, out, query, kind_filter);
        }
    }
}

fn expand_external_glob(
    docs: &rustdoc_types::Crate,
    externals: &HashMap<String, rustdoc_types::Crate>,
    target_id: Id,
    prefix: &str,
    out: &mut Vec<Match>,
    query: Option<&str>,
    kind_filter: Option<ItemKind>,
) {
    let Some(summary) = docs.paths.get(&target_id) else {
        return;
    };
    let Some(ext_crate) = docs.external_crates.get(&summary.crate_id) else {
        return;
    };
    let Some(ext_docs) = externals.get(&ext_crate.name) else {
        // External crate not fetched; we can't enumerate its items.
        return;
    };
    // Find the equivalent module in the external crate by matching the path.
    let Some(ext_module_id) = ext_docs
        .paths
        .iter()
        .find_map(|(id, s)| (s.path == summary.path).then_some(*id))
    else {
        return;
    };

    let ext_name = ext_crate.name.clone();
    let ext_version = ext_crate
        .html_root_url
        .as_deref()
        .and_then(parse_version_from_docs_rs_url);

    let len_before = out.len();
    let mut ext_visited = HashSet::new();
    expand_module(
        ext_docs,
        externals,
        ext_module_id,
        prefix,
        &mut ext_visited,
        out,
        query,
        kind_filter,
    );

    // Items native to the external crate come back with source_crate=None
    // (their crate_id is 0 in the external's own namespace). Attribute them
    // to the external crate we just enumerated.
    for m in out[len_before..].iter_mut() {
        if let Some(rx) = m.reexport.as_mut() {
            if rx.source_crate.is_none() {
                rx.source_crate = Some(ext_name.clone());
                rx.source_version = ext_version.clone();
            }
        }
    }
}

fn emit_reexport(
    docs: &rustdoc_types::Crate,
    target: &rustdoc_types::Item,
    name: &str,
    prefix: &str,
    out: &mut Vec<Match>,
    query: Option<&str>,
    kind_filter: Option<ItemKind>,
) {
    let Some(resolved) = resolve_through_uses(docs, target) else {
        return;
    };
    let kind: ItemKind = resolved.inner.item_kind().into();
    if kind_filter.is_some_and(|filter| filter != kind) {
        return;
    }
    let path = join_path(prefix, name);
    if !matches_query(query, name, &path) {
        return;
    }
    out.push(Match {
        id: resolved.id,
        name: name.to_string(),
        path,
        kind,
        reexport: Some(reexport_info(docs, resolved)),
    });
}

fn emit_external_reexport(
    docs: &rustdoc_types::Crate,
    target_id: Id,
    name: &str,
    prefix: &str,
    out: &mut Vec<Match>,
    query: Option<&str>,
    kind_filter: Option<ItemKind>,
) {
    let Some(summary) = docs.paths.get(&target_id) else {
        return;
    };
    let kind: ItemKind = summary.kind.clone().into();
    if kind_filter.is_some_and(|filter| filter != kind) {
        return;
    }
    let path = join_path(prefix, name);
    if !matches_query(query, name, &path) {
        return;
    }

    let ext = docs.external_crates.get(&summary.crate_id);
    out.push(Match {
        id: target_id,
        name: name.to_string(),
        path,
        kind,
        reexport: Some(Reexport {
            source_crate: ext.map(|e| e.name.clone()),
            source_version: ext
                .and_then(|e| e.html_root_url.as_deref())
                .and_then(parse_version_from_docs_rs_url),
        }),
    });
}

/// External crate referenced by a glob re-export whose target lives outside
/// `docs.index`. To fully expand such a re-export you'd need to fetch this
/// crate's own rustdoc JSON.
#[derive(Debug, Serialize)]
pub(crate) struct NeededCrate {
    pub(crate) name: String,
    /// Version parsed from `html_root_url` if it looks like a docs.rs URL.
    /// `None` for stdlib, path/git deps, or unusual root URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<String>,
    /// Raw `html_root_url` from rustdoc, kept for debugging / non-docs.rs URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) html_root_url: Option<String>,
}

/// Crates whose rustdoc JSON would be needed to fully expand the glob
/// re-exports of `docs` that target external crates.
pub(crate) fn needed_crates(docs: &rustdoc_types::Crate) -> Vec<NeededCrate> {
    let ids: HashSet<u32> = docs
        .index
        .values()
        .filter_map(|item| match &item.inner {
            ItemEnum::Use(u) if u.is_glob => u.id,
            _ => None,
        })
        .filter(|target_id| !docs.index.contains_key(target_id))
        .filter_map(|target_id| docs.paths.get(&target_id).map(|s| s.crate_id))
        .collect();

    let mut result: Vec<NeededCrate> = ids
        .into_iter()
        .filter_map(|id| docs.external_crates.get(&id))
        .map(|ec| NeededCrate {
            name: ec.name.clone(),
            version: ec
                .html_root_url
                .as_deref()
                .and_then(parse_version_from_docs_rs_url),
            html_root_url: ec.html_root_url.clone(),
        })
        .collect();
    result.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
    result
}

fn resolve_through_uses<'a>(
    docs: &'a rustdoc_types::Crate,
    item: &'a rustdoc_types::Item,
) -> Option<&'a rustdoc_types::Item> {
    let mut current = item;
    for _ in 0..32 {
        match &current.inner {
            ItemEnum::Use(u) => {
                let target_id = u.id?;
                current = docs.index.get(&target_id)?;
            }
            _ => return Some(current),
        }
    }
    None
}

fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}

fn reexport_info(docs: &rustdoc_types::Crate, target: &rustdoc_types::Item) -> Reexport {
    if target.crate_id == 0 {
        return Reexport {
            source_crate: None,
            source_version: None,
        };
    }
    let ext = docs.external_crates.get(&target.crate_id);
    Reexport {
        source_crate: ext.map(|e| e.name.clone()),
        source_version: ext
            .and_then(|e| e.html_root_url.as_deref())
            .and_then(parse_version_from_docs_rs_url),
    }
}

fn parse_version_from_docs_rs_url(url: &str) -> Option<String> {
    // Expected shape: https://docs.rs/<crate>/<version>/<crate>/
    let rest = url.strip_prefix("https://docs.rs/")?;
    let mut parts = rest.split('/');
    parts.next()?; // crate
    let version = parts.next()?;
    (!version.is_empty()).then(|| version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::docs_fixture;
    use anyhow::Result;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_list_modules() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let results = search(&docs, &HashMap::new(), None, Some(ItemKind::Module), None);

        assert!(results.iter().all(|m| m.kind == ItemKind::Module));

        assert_eq!(
            results.into_iter().map(|m| m.path).collect::<Vec<_>>(),
            vec![
                "axum",
                "axum::body",
                "axum::error_handling",
                "axum::error_handling::future",
                "axum::extract",
                "axum::extract::connect_info",
                "axum::extract::multipart",
                "axum::extract::path",
                "axum::extract::rejection",
                "axum::extract::ws",
                "axum::extract::ws::close_code",
                "axum::extract::ws::rejection",
                "axum::handler",
                "axum::handler::future",
                "axum::http",
                "axum::middleware",
                "axum::middleware::future",
                "axum::response",
                "axum::response::sse",
                "axum::routing",
                "axum::routing::future",
                "axum::routing::method_routing",
                "axum::serve",
                "axum::test_helpers",
                "test_client",
            ]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_modules_filtered() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let results = search(
            &docs,
            &HashMap::new(),
            Some("extract"),
            Some(ItemKind::Module),
            None,
        );

        assert!(results.iter().all(|m| m.kind == ItemKind::Module));

        assert_eq!(
            results.into_iter().map(|m| m.path).collect::<Vec<_>>(),
            vec![
                "axum::extract",
                "axum::extract::connect_info",
                "axum::extract::multipart",
                "axum::extract::path",
                "axum::extract::rejection",
                "axum::extract::ws",
                "axum::extract::ws::close_code",
                "axum::extract::ws::rejection",
            ]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_traits() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let needed: HashSet<_> = docs
            .index
            .values()
            .filter_map(|item| match &item.inner {
                ItemEnum::Use(u) if u.is_glob => u.id,
                _ => None,
            })
            .filter(|target_id| !docs.index.contains_key(target_id))
            // .filter_map(|target_id| docs.paths.get(&target_id).map(|s| s.crate_id))
            .filter_map(|target_id| docs.paths.get(&target_id))
            .collect();

        dbg!(&needed);

        let results = search(&docs, &HashMap::new(), None, Some(ItemKind::Trait), None);

        assert!(results.iter().all(|m| m.kind == ItemKind::Trait));

        assert_eq!(
            results.into_iter().map(|m| m.path).collect::<Vec<_>>(),
            vec![
                "axum::RequestExt",
                "axum::RequestPartsExt",
                "axum::ServiceExt",
                "axum::body::HttpBody",
                "axum::extract::FromRef",
                "axum::extract::FromRequest",
                "axum::extract::FromRequestParts",
                "axum::extract::OptionalFromRequest",
                "axum::extract::OptionalFromRequestParts",
                "axum::extract::connect_info::Connected",
                "axum::extract::ws::OnFailedUpgrade",
                "axum::handler::Handler",
                "axum::handler::HandlerWithoutStateExt",
                "axum::middleware::IntoMapRequestResult",
                "axum::middleware::map_request::IntoMapRequestResult",
                "axum::middleware::map_request::private::Sealed",
                "axum::response::IntoResponse",
                "axum::response::IntoResponseParts",
                "axum::serve::Listener",
                "axum::serve::ListenerExt",
                "axum::serve::listener::Listener",
                "axum::serve::listener::ListenerExt",
                "axum::service_ext::ServiceExt",
            ]
        );

        panic!();

        Ok(())
    }
}
