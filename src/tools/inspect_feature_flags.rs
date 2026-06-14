use crate::{
    client::inspect_feature_flags, context::Context, tools::render_response, types::semver::Version,
};
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

#[tracing::instrument(
    name = "tool.inspect_feature_flags",
    skip(context),
    fields(krate = %args.krate, version = %args.version.as_ref()),
)]
pub(crate) async fn handle(
    context: &Context,
    args: InspectFeatureFlagsArgs,
) -> Result<CallToolResult, McpError> {
    let features =
        inspect_feature_flags::inspect_feature_flags(context, &args.krate, args.version.as_ref())
            .await?;

    render_response(InspectFeatureFlagsResult { features })
}
