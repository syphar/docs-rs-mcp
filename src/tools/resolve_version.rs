use crate::{client::status::get_docs_status, config::Config, semver_types::VersionReq};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ResolveVersionArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Semver requirement (e.g. "1", "^1.5", "0.14.0") or "*". Defaults to "*".
    #[serde(default)]
    pub(crate) req: VersionReq,
}

pub(crate) async fn handle(
    config: &Config,
    args: ResolveVersionArgs,
) -> Result<CallToolResult, McpError> {
    let status = get_docs_status(config, &args.krate, args.req.as_ref())
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .ok_or_else(|| McpError::resource_not_found("crate or version not found", None))?;

    Ok(CallToolResult::structured(
        serde_json::to_value(&status)
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
