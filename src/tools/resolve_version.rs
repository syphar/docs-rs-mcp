use crate::docs_rs::get_docs_status;
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ResolveVersionArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Semver requirement (e.g. "1", "^1.5", "0.14.0") or "*". Defaults to "*".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) req: Option<String>,
}

pub(crate) async fn handle(args: ResolveVersionArgs) -> Result<CallToolResult, McpError> {
    let version_req: semver::VersionReq =
        args.req
            .as_deref()
            .unwrap_or("*")
            .parse()
            .map_err(|err: semver::Error| {
                McpError::invalid_params(
                    format!("invalid semver version requirement: {}", err),
                    Some(serde_json::json!({ "req": args.req })),
                )
            })?;

    let status = get_docs_status(&args.krate, &version_req)
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .ok_or_else(|| McpError::resource_not_found("crate or version not found", None))?;

    Ok(CallToolResult::structured(
        serde_json::to_value(&status)
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
