use crate::{
    client::{
        crate_metadata,
        get_docs::{LoadedDocs, get_docs},
        inspect_feature_flags, manifest_dependencies,
        render::render_item_signature,
    },
    context::Context,
    errors::Error,
    types::rustdoc_types::ItemKind,
};
use rustdoc_types::ItemEnum;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
pub(crate) struct ApiItem {
    pub(crate) path: String,
    pub(crate) kind: ItemKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) signature: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SignatureChange {
    pub(crate) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) after: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ValueChange {
    pub(crate) key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) before: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) after: Option<Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct VersionComparison {
    pub(crate) items_added: Vec<ApiItem>,
    pub(crate) items_removed: Vec<ApiItem>,
    pub(crate) signatures_changed: Vec<SignatureChange>,
    pub(crate) features_changed: Vec<ValueChange>,
    pub(crate) dependencies_changed: Vec<ValueChange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) msrv_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) msrv_after: Option<String>,
}

struct ApiSnapshot {
    item: ApiItem,
    structured: Value,
}

pub(crate) async fn compare_versions(
    context: &Context,
    krate: &str,
    from: &semver::Version,
    to: &semver::Version,
    target: &str,
) -> Result<
    (
        VersionComparison,
        std::sync::Arc<LoadedDocs>,
        std::sync::Arc<LoadedDocs>,
    ),
    Error,
> {
    let (from_docs, to_docs) = tokio::try_join!(
        get_docs(context, krate, from, Some(target)),
        get_docs(context, krate, to, Some(target)),
    )?;
    let (from_features, to_features, from_deps, to_deps, from_meta, to_meta) = tokio::try_join!(
        inspect_feature_flags::inspect_feature_flags(context, krate, from),
        inspect_feature_flags::inspect_feature_flags(context, krate, to),
        manifest_dependencies::manifest_dependencies(context, krate, from),
        manifest_dependencies::manifest_dependencies(context, krate, to),
        crate_metadata::crate_metadata(context, krate, from),
        crate_metadata::crate_metadata(context, krate, to),
    )?;

    let before_api = api_snapshot(&from_docs)?;
    let after_api = api_snapshot(&to_docs)?;
    let before_paths: BTreeSet<_> = before_api.keys().cloned().collect();
    let after_paths: BTreeSet<_> = after_api.keys().cloned().collect();

    let items_added = after_paths
        .difference(&before_paths)
        .filter_map(|path| after_api.get(path).map(|snapshot| api_item(&snapshot.item)))
        .collect();
    let items_removed = before_paths
        .difference(&after_paths)
        .filter_map(|path| {
            before_api
                .get(path)
                .map(|snapshot| api_item(&snapshot.item))
        })
        .collect();
    let signatures_changed = before_paths
        .intersection(&after_paths)
        .filter_map(|path| {
            let before = before_api.get(path)?;
            let after = after_api.get(path)?;
            (before.structured != after.structured).then(|| SignatureChange {
                path: path.clone(),
                before: before.item.signature.clone(),
                after: after.item.signature.clone(),
            })
        })
        .collect();

    let features_changed = compare_values(
        keyed_values(from_features, |feature| feature.name.clone())?,
        keyed_values(to_features, |feature| feature.name.clone())?,
    );
    let dependencies_changed = compare_values(
        keyed_values(from_deps, dependency_key)?,
        keyed_values(to_deps, dependency_key)?,
    );

    Ok((
        VersionComparison {
            items_added,
            items_removed,
            signatures_changed,
            features_changed,
            dependencies_changed,
            msrv_before: from_meta.rust_version,
            msrv_after: to_meta.rust_version,
        },
        from_docs,
        to_docs,
    ))
}

fn api_snapshot(docs: &rustdoc_types::Crate) -> Result<BTreeMap<String, ApiSnapshot>, Error> {
    docs.index
        .values()
        .filter(|item| item.crate_id == 0 && !matches!(item.inner, ItemEnum::Use(_)))
        .filter_map(|item| {
            let path = docs.paths.get(&item.id)?.path.join("::");
            Some((path, item))
        })
        .map(|(path, item)| {
            Ok((
                path.clone(),
                ApiSnapshot {
                    item: ApiItem {
                        path,
                        kind: item.inner.item_kind().into(),
                        signature: render_item_signature(item.name.as_deref(), &item.inner),
                    },
                    structured: serde_json::to_value(&item.inner).map_err(anyhow::Error::from)?,
                },
            ))
        })
        .collect()
}

fn api_item(item: &ApiItem) -> ApiItem {
    ApiItem {
        path: item.path.clone(),
        kind: item.kind,
        signature: item.signature.clone(),
    }
}

fn keyed_values<T: Serialize>(
    values: Vec<T>,
    key: impl Fn(&T) -> String,
) -> Result<BTreeMap<String, Value>, Error> {
    values
        .into_iter()
        .map(|value| {
            let key = key(&value);
            Ok((
                key,
                serde_json::to_value(value).map_err(anyhow::Error::from)?,
            ))
        })
        .collect()
}

fn dependency_key(dependency: &manifest_dependencies::Dependency) -> String {
    format!(
        "{}:{:?}:{}",
        dependency.name,
        dependency.kind,
        dependency.target.as_deref().unwrap_or("")
    )
}

fn compare_values(
    before: BTreeMap<String, Value>,
    after: BTreeMap<String, Value>,
) -> Vec<ValueChange> {
    before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|key| {
            let before = before.get(key);
            let after = after.get(key);
            (before != after).then(|| ValueChange {
                key: key.clone(),
                before: before.cloned(),
                after: after.cloned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixture, test_env};
    use anyhow::Result;

    #[tokio::test]
    async fn identical_versions_have_no_changes() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let docs = fixture("axum_0.8.9.json.zst")?;
        let source = fixture("axum-0.8.9.crate")?;

        let _docs_mock = env
            .server
            .mock("GET", "/crate/axum/0.8.9/test-target/json.zst")
            .with_status(200)
            .with_body_from_file(docs)
            .create();
        let _source_mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(source)
            .create();

        let (comparison, _, _) =
            compare_versions(env.context(), "axum", &version, &version, "test-target").await?;

        assert!(comparison.items_added.is_empty());
        assert!(comparison.items_removed.is_empty());
        assert!(comparison.signatures_changed.is_empty());
        assert!(comparison.features_changed.is_empty());
        assert!(comparison.dependencies_changed.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn compares_axum_0_8_8_to_0_8_9() -> Result<()> {
        let mut env = test_env().await?;
        let from = semver::Version::new(0, 8, 8);
        let to = semver::Version::new(0, 8, 9);

        let _from_docs = env
            .server
            .mock("GET", "/crate/axum/0.8.8/test-target/json.zst")
            .with_status(200)
            .with_body_from_file(fixture("axum_0.8.8.json.zst")?)
            .create();
        let _to_docs = env
            .server
            .mock("GET", "/crate/axum/0.8.9/test-target/json.zst")
            .with_status(200)
            .with_body_from_file(fixture("axum_0.8.9.json.zst")?)
            .create();
        let _from_source = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.8.crate")
            .with_status(200)
            .with_body_from_file(fixture("axum-0.8.8.crate")?)
            .create();
        let _to_source = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(fixture("axum-0.8.9.crate")?)
            .create();

        let (comparison, from_docs, to_docs) =
            compare_versions(env.context(), "axum", &from, &to, "test-target").await?;

        assert_eq!(comparison.msrv_before.as_deref(), Some("1.78"));
        assert_eq!(comparison.msrv_after.as_deref(), Some("1.80"));
        assert!(comparison.items_added.is_empty());
        assert!(comparison.items_removed.is_empty());
        assert!(comparison.features_changed.is_empty());
        assert!(
            comparison
                .signatures_changed
                .iter()
                .all(|change| change.path == "axum" || change.path.starts_with("axum::")),
            "non-axum changes: {:?}",
            comparison
                .signatures_changed
                .iter()
                .filter(|change| change.path != "axum" && !change.path.starts_with("axum::"))
                .map(|change| &change.path)
                .take(10)
                .collect::<Vec<_>>()
        );

        let macros = comparison
            .dependencies_changed
            .iter()
            .find(|change| change.key == "axum-macros:Normal:")
            .expect("axum-macros dependency changed");
        assert_eq!(
            macros
                .before
                .as_ref()
                .and_then(|value| value["req"].as_str()),
            Some("0.5.0")
        );
        assert_eq!(
            macros
                .after
                .as_ref()
                .and_then(|value| value["req"].as_str()),
            Some("0.5.1")
        );
        assert!(!from_docs.target_fallback);
        assert!(!to_docs.target_fallback);
        Ok(())
    }
}
