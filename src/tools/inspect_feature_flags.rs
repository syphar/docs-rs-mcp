use crate::{client::inspect_feature_flags, context::Context, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct InspectFeatureFlagsArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
}

#[derive(Debug, Serialize)]
struct InspectFeatureFlagsResult {
    features: Vec<inspect_feature_flags::Feature>,
}

pub(crate) async fn handle(
    config: &Context,
    args: InspectFeatureFlagsArgs,
) -> Result<CallToolResult, McpError> {
    let features =
        inspect_feature_flags::inspect_feature_flags(config, &args.krate, args.version.as_ref())
            .await
            .map_err(|err| McpError::internal_error(err.to_string(), None))?
            .ok_or_else(|| {
                McpError::resource_not_found("crate or version not found on crates.io", None)
            })?;

    Ok(CallToolResult::structured(
        serde_json::to_value(InspectFeatureFlagsResult { features })
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
