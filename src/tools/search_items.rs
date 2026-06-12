use crate::{
    client::{
        docs::get_docs,
        search_items::{SearchItemMatch, search_items},
    },
    config::Config,
    types::{rustdoc_types::ItemKind, semver::Version},
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SearchItemsArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Exact crate version to load rustdoc JSON for (e.g. "1.2.3"). This is
    /// not a semver requirement — ranges like "^1.2" or "*" are not accepted.
    /// Use the `resolve_version` tool first to turn a requirement into a
    /// concrete version. To find the version currently used in a local
    /// project, run `cargo tree -p <crate>` or `cargo pkgid <crate>` in the
    /// project directory, or read it from `Cargo.lock`.
    pub(crate) version: Version,
    /// Search text matched against item names and paths.
    pub(crate) query: String,
    /// Optional item kind filter, e.g. "struct", "enum", "trait", "function", "module".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<ItemKind>,
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

pub(crate) async fn handle(
    config: &Config,
    args: SearchItemsArgs,
) -> Result<CallToolResult, McpError> {
    let docs = get_docs(config, &args.krate, args.version.as_ref())
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?;

    let items = search_items(&docs, &args.query, args.kind, args.limit);

    Ok(CallToolResult::structured(
        serde_json::to_value(SearchItemsResult { items })
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
