use crate::{client::status::get_docs_status, context::Config, types::semver::VersionReq};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ResolveVersionArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Cargo-style semver requirement. Bare versions are caret requirements:
    /// "1.2.3" means ">=1.2.3, <2.0.0" (compatible), not an exact match.
    /// Use "=1.2.3" for an exact version. Other examples: "1", "^1.5",
    /// "~1.2", ">=1.2, <1.5", "*". Defaults to "*" (latest).
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
