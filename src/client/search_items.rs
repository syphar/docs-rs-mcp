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
    query: Option<&str>,
    kind_filter: Option<ItemKind>,
    limit: Option<usize>,
) -> Vec<Match> {
    let query = query.map(|q| q.to_lowercase());

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

            if let Some(query) = &query {
                haystack.contains(query).then_some(Match {
                    id: item.id,
                    name,
                    path,
                    kind,
                })
            } else {
                Some(Match {
                    id: item.id,
                    name,
                    path,
                    kind,
                })
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::docs_fixture;
    use anyhow::Result;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_list_modules() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let results = search(&docs, None, Some(ItemKind::Module), None);

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

        let results = search(&docs, Some("extract"), Some(ItemKind::Module), None);

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
}
