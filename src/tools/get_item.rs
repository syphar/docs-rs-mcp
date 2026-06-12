use crate::{
    client::{
        get_docs::get_docs,
        get_item::{self, Verbosity},
    },
    context::Config,
    types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

const HOST_TARGET: &str = env!("BUILD_TARGET");

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct GetItemArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Exact crate version (e.g. "1.2.3"). Not a semver requirement — call
    /// `resolve_version` first to turn a requirement into a concrete version.
    pub(crate) version: Version,
    /// Fully-qualified path of the item, including the crate name
    /// (e.g. `"axum::Router"` or `"axum::routing::Router"`). Re-export paths
    /// are accepted: `axum::Router` resolves to the canonical
    /// `axum::routing::Router`. The `path` field on the result is always the
    /// canonical one.
    pub(crate) path: String,
    /// How much detail to return. Defaults to `"full"`.
    ///   - `"signature"`: structured signature only (kind, generics,
    ///     fields/variants, function decl, etc.). No `docs`, no `examples`.
    ///   - `"full"`: signature + doc string + extracted Rust code blocks.
    #[serde(default)]
    pub(crate) verbosity: Verbosity,
    /// Target triple. Same semantics as `search_items.target`: defaults to
    /// the host the server was compiled for; falls back to the crate's
    /// docs.rs-default target on 404.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

pub(crate) async fn handle(config: &Config, args: GetItemArgs) -> Result<CallToolResult, McpError> {
    let target = args.target.as_deref().unwrap_or(HOST_TARGET);
    let docs = get_docs(config, &args.krate, args.version.as_ref(), Some(target))
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .ok_or_else(|| {
            McpError::resource_not_found("crate or version not found on docs.rs", None)
        })?;

    let path: Vec<String> = args.path.split("::").map(str::to_string).collect();

    let record = get_item::get_item(&docs, &path, args.verbosity)
        .ok_or_else(|| McpError::resource_not_found("item not found at the given path", None))?;

    Ok(CallToolResult::structured(
        serde_json::to_value(record)
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
