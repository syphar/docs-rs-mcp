use crate::types::rustdoc_types::ItemKind;
use rustdoc_types::Id;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Match {
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: ItemKind,
}

pub(crate) fn search(
    docs: &rustdoc_types::Crate,
    query: &str,
    kind_filter: Option<ItemKind>,
    limit: Option<usize>,
) -> Vec<Match> {
    let query = query.to_lowercase();

    let mut matches = docs
        .index
        .values()
        .filter_map(|item| {
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
            let haystack = format!("{name} {path}").to_lowercase();

            haystack.contains(&query).then_some(Match {
                id: item.id,
                name,
                path,
                kind,
            })
        })
        .collect::<Vec<_>>();

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
