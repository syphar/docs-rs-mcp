use crate::{
    client::{get_docs::get_docs, list_methods},
    context::Context,
    types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

const HOST_TARGET: &str = env!("BUILD_TARGET");

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ListMethodsArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
    /// Fully-qualified path of the type whose methods to list, including the
    /// crate name (e.g. `"axum::routing::Router"` or `"axum::Router"`).
    /// Re-export paths are resolved to the canonical type.
    pub(crate) type_path: String,
    /// Target triple. Same semantics as `search_items.target`: defaults to
    /// the host the server was compiled for; falls back to the crate's
    /// docs.rs-default target on 404.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListMethodsResult {
    methods: Vec<list_methods::Method>,
}

pub(crate) async fn handle(
    context: &Context,
    args: ListMethodsArgs,
) -> Result<CallToolResult, McpError> {
    let target = args.target.as_deref().unwrap_or(HOST_TARGET);
    let docs = get_docs(context, &args.krate, args.version.as_ref(), Some(target))
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .ok_or_else(|| {
            McpError::resource_not_found("crate or version not found on docs.rs", None)
        })?;

    let path: Vec<&str> = args.type_path.split("::").collect();

    let methods = list_methods::list_methods(&docs, &path)
        .ok_or_else(|| McpError::resource_not_found("type not found at the given path", None))?;

    Ok(CallToolResult::structured(
        serde_json::to_value(ListMethodsResult { methods })
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
