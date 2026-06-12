use crate::{client::find_examples, context::Context, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct FindExamplesArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
    /// When `true`, include the full source of each example file. Defaults
    /// to `false` — returns just the file paths and names.
    #[serde(default)]
    pub(crate) include_content: bool,
}

#[derive(Debug, Serialize)]
struct FindExamplesResult {
    examples: Vec<find_examples::Example>,
}

pub(crate) async fn handle(
    config: &Context,
    args: FindExamplesArgs,
) -> Result<CallToolResult, McpError> {
    let examples = find_examples::find_examples(
        config,
        &args.krate,
        args.version.as_ref(),
        args.include_content,
    )
    .await
    .map_err(|err| McpError::internal_error(err.to_string(), None))?
    .ok_or_else(|| McpError::resource_not_found("crate or version not found on crates.io", None))?;

    Ok(CallToolResult::structured(
        serde_json::to_value(FindExamplesResult { examples })
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
