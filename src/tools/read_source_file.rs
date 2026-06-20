use crate::{client::source, context::Context, tools::render_response, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ReadSourceFileArgs {
    pub(crate) krate: String,
    pub(crate) version: Version,
    /// Path relative to the published crate root.
    pub(crate) path: String,
    #[serde(default = "default_start_line")]
    pub(crate) start_line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) end_line: Option<usize>,
    #[serde(default = "default_max_chars")]
    pub(crate) max_chars: usize,
}

fn default_start_line() -> usize {
    1
}

fn default_max_chars() -> usize {
    30_000
}

pub(crate) async fn handle(
    context: &Context,
    args: ReadSourceFileArgs,
) -> Result<CallToolResult, McpError> {
    let file = source::read_source_file(
        context,
        &args.krate,
        args.version.as_ref(),
        &args.path,
        args.start_line,
        args.end_line,
        args.max_chars,
    )
    .await?;
    render_response(file)
}
