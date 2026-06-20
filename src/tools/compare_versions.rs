use crate::{
    client::{compare_versions, get_docs::TargetResolution},
    context::Context,
    tools::render_response,
    types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

const HOST_TARGET: &str = env!("BUILD_TARGET");

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct CompareVersionsArgs {
    pub(crate) krate: String,
    pub(crate) from_version: Version,
    pub(crate) to_version: Version,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Serialize)]
struct CompareVersionsResult {
    from_target: TargetResolution,
    to_target: TargetResolution,
    #[serde(flatten)]
    comparison: compare_versions::VersionComparison,
}

pub(crate) async fn handle(
    context: &Context,
    args: CompareVersionsArgs,
) -> Result<CallToolResult, McpError> {
    let target = args.target.as_deref().unwrap_or(HOST_TARGET);
    let (comparison, from_docs, to_docs) = compare_versions::compare_versions(
        context,
        &args.krate,
        args.from_version.as_ref(),
        args.to_version.as_ref(),
        target,
    )
    .await?;

    render_response(CompareVersionsResult {
        from_target: from_docs.target_resolution(),
        to_target: to_docs.target_resolution(),
        comparison,
    })
}
