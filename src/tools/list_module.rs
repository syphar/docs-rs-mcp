use crate::{
    client::{get_docs::get_docs, list_module, search_items::UnexpandedExternalGlob},
    context::Context,
    types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

/// Same host-default convention as `search_items` — see that module for the
/// rationale. `get_docs` falls back to the crate's docs.rs-default target on
/// 404.
const HOST_TARGET: &str = env!("BUILD_TARGET");

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ListModuleArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Exact crate version (e.g. "1.2.3"). Not a semver requirement — call
    /// `resolve_version` first to turn a requirement into a concrete version.
    pub(crate) version: Version,
    /// Module path to list, fully qualified including the crate name
    /// (e.g. `"axum::extract"`). Omit to list the crate root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
    /// Target triple. Same semantics as `search_items.target`: defaults to
    /// the host the server was compiled for; override when the user's
    /// project targets something else. Falls back to the crate's docs.rs
    /// default target on 404.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListModuleResult {
    entries: Vec<list_module::Entry>,
    /// Glob re-exports at this module level that target external crates.
    /// Follow up by calling `list_module` (or `search_items`) against
    /// `source_crate` + `source_version`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unexpanded_external_globs: Vec<UnexpandedExternalGlob>,
}

pub(crate) async fn handle(
    config: &Context,
    args: ListModuleArgs,
) -> Result<CallToolResult, McpError> {
    let target = args.target.as_deref().unwrap_or(HOST_TARGET);
    let docs = get_docs(config, &args.krate, args.version.as_ref(), Some(target))
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .ok_or_else(|| {
            McpError::resource_not_found("crate or version not found on docs.rs", None)
        })?;

    let path_vec: Option<Vec<String>> = args
        .path
        .as_deref()
        .map(|p| p.split("::").map(str::to_string).collect());

    let listing = list_module::list_module(&docs, path_vec.as_deref())
        .ok_or_else(|| McpError::resource_not_found("module not found at the given path", None))?;

    Ok(CallToolResult::structured(
        serde_json::to_value(ListModuleResult {
            entries: listing.entries,
            unexpanded_external_globs: listing.unexpanded_external_globs,
        })
        .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
