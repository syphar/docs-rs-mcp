use crate::{client::source, context::Context, tools::render_response, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SearchSourceArgs {
    pub(crate) krate: String,
    pub(crate) version: Version,
    pub(crate) query: String,
    /// Wildcard path filter. `*` matches any characters, including `/`.
    #[serde(default = "default_glob")]
    pub(crate) path_glob: String,
    #[serde(default = "default_limit")]
    pub(crate) limit: usize,
    /// Lines before and after each match, capped at 5.
    #[serde(default = "default_context_lines")]
    pub(crate) context_lines: usize,
}

fn default_glob() -> String {
    "**/*.rs".into()
}

fn default_limit() -> usize {
    20
}

fn default_context_lines() -> usize {
    2
}

#[derive(Debug, Serialize)]
struct SearchSourceResult {
    matches: Vec<source::SourceMatch>,
}

pub(crate) async fn handle(
    context: &Context,
    args: SearchSourceArgs,
) -> Result<CallToolResult, McpError> {
    let matches = source::search_source(
        context,
        &args.krate,
        args.version.as_ref(),
        &args.query,
        &args.path_glob,
        args.limit.min(100),
        args.context_lines.min(5),
    )
    .await?;
    render_response(SearchSourceResult { matches })
}
