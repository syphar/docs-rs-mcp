use crate::{client::versions, context::Context, tools::render_response};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ListVersionsArgs {
    pub(crate) krate: String,
    #[serde(default = "default_limit")]
    pub(crate) limit: usize,
    #[serde(default)]
    pub(crate) include_yanked: bool,
    #[serde(default)]
    pub(crate) include_prerelease: bool,
    /// Check docs.rs availability for each returned version.
    #[serde(default)]
    pub(crate) check_docs: bool,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize)]
struct ListVersionsResult {
    versions: Vec<versions::VersionInfo>,
}

pub(crate) async fn handle(
    context: &Context,
    args: ListVersionsArgs,
) -> Result<CallToolResult, McpError> {
    let versions = versions::list_versions(
        context,
        &args.krate,
        args.limit.min(100),
        args.include_yanked,
        args.include_prerelease,
        args.check_docs,
    )
    .await?;
    render_response(ListVersionsResult { versions })
}
