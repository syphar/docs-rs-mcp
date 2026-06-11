use crate::{config::Config, rustdoc_json::get_docs};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SearchItemsArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Concrete crate version to load rustdoc JSON for.
    pub(crate) version: String,
    /// Search text matched against item names and paths.
    pub(crate) query: String,
    /// Optional item kind filter, e.g. "struct", "enum", "trait", "function", "module".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
    /// Maximum number of matches to return. Defaults to 20.
    #[serde(default = "default_limit")]
    pub(crate) limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize)]
struct SearchItemsResult {
    items: Vec<SearchItemMatch>,
}

#[derive(Debug, Serialize)]
struct SearchItemMatch {
    id: String,
    name: String,
    path: String,
    kind: String,
}

pub(crate) async fn handle(
    config: &Config,
    args: SearchItemsArgs,
) -> Result<CallToolResult, McpError> {
    let version = args.version.parse().map_err(|err: semver::Error| {
        McpError::invalid_params(
            format!("invalid semver version: {}", err),
            Some(serde_json::json!({ "version": args.version })),
        )
    })?;

    let kind_filter = args.kind.as_deref().map(normalize_kind);
    let query = args.query.to_lowercase();
    let docs = get_docs(config, &args.krate, &version)
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?;

    let mut matches = docs
        .index
        .values()
        .filter_map(|item| {
            let kind = serde_json::to_value(item.inner.item_kind()).ok()?;
            let kind = kind.as_str()?;
            if kind_filter.as_deref().is_some_and(|filter| filter != kind) {
                return None;
            }

            let path = docs
                .paths
                .get(&item.id)
                .map(|summary| summary.path.join("::"))
                .or_else(|| item.name.clone())?;
            let name = item.name.clone().unwrap_or_default();
            let haystack = format!("{name} {path}").to_lowercase();

            haystack.contains(&query).then_some(SearchItemMatch {
                id: item.id.0.to_string(),
                name,
                path,
                kind: kind.to_string(),
            })
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    matches.truncate(args.limit);

    Ok(CallToolResult::structured(
        serde_json::to_value(SearchItemsResult { items: matches })
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}

fn normalize_kind(kind: &str) -> String {
    let kind = kind.to_ascii_lowercase();
    match kind.as_str() {
        "fn" => "function".to_string(),
        "mod" => "module".to_string(),
        _ => kind,
    }
}
