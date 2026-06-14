use crate::{client::readme, context::Context, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ReadmeArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
}

#[tracing::instrument(
    name = "tool.readme",
    skip(context),
    fields(krate = %args.krate, version = %args.version.as_ref()),
)]
pub(crate) async fn handle(
    context: &Context,
    args: ReadmeArgs,
) -> Result<CallToolResult, McpError> {
    let readme = readme::readme(context, &args.krate, args.version.as_ref())
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .ok_or_else(|| {
            McpError::resource_not_found("no README file found in this crate's source", None)
        })?;

    Ok(CallToolResult::structured(
        serde_json::to_value(readme)
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
