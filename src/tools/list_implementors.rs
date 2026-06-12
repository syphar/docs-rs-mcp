use crate::{
    client::{get_docs::get_docs, list_implementors},
    config::Config,
    types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

const HOST_TARGET: &str = env!("BUILD_TARGET");

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ListImplementorsArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
    /// Fully-qualified path of the trait whose implementors to list, including
    /// the crate name (e.g. `"axum::handler::Handler"`). The trait must live
    /// *in this crate* — rustdoc JSON only knows about impls visible in the
    /// crate being queried.
    pub(crate) trait_path: String,
    /// Target triple. Same semantics as `search_items.target`: defaults to
    /// the host the server was compiled for; falls back to the crate's
    /// docs.rs-default target on 404.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListImplementorsResult {
    implementors: Vec<list_implementors::Implementor>,
}

pub(crate) async fn handle(
    config: &Config,
    args: ListImplementorsArgs,
) -> Result<CallToolResult, McpError> {
    let target = args.target.as_deref().unwrap_or(HOST_TARGET);
    let docs = get_docs(config, &args.krate, args.version.as_ref(), Some(target))
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .ok_or_else(|| {
            McpError::resource_not_found("crate or version not found on docs.rs", None)
        })?;

    let path: Vec<String> = args.trait_path.split("::").map(str::to_string).collect();

    let implementors = list_implementors::list_implementors(&docs, &path).ok_or_else(|| {
        McpError::resource_not_found("no trait found at the given path", None)
    })?;

    Ok(CallToolResult::structured(
        serde_json::to_value(ListImplementorsResult { implementors })
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
