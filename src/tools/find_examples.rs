use crate::{
    client::find_examples, context::Context, tools::render_response, types::semver::Version,
};
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

#[tracing::instrument(
    name = "tool.find_examples",
    skip(context),
    fields(
        krate = %args.krate,
        version = %args.version.as_ref(),
        include_content = args.include_content,
    ),
)]
pub(crate) async fn handle(
    context: &Context,
    args: FindExamplesArgs,
) -> Result<CallToolResult, McpError> {
    let examples = find_examples::find_examples(
        context,
        &args.krate,
        args.version.as_ref(),
        args.include_content,
    )
    .await?;

    render_response(FindExamplesResult { examples })
}
