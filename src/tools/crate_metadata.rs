use crate::{
    client::crate_metadata, context::Context, tools::render_response, types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct CrateMetadataArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
}

#[tracing::instrument(
    name = "tool.crate_metadata",
    skip(context),
    fields(krate = %args.krate, version = %args.version.as_ref()),
)]
pub(crate) async fn handle(
    context: &Context,
    args: CrateMetadataArgs,
) -> Result<CallToolResult, McpError> {
    let meta = crate_metadata::crate_metadata(context, &args.krate, args.version.as_ref()).await?;

    render_response(meta)
}
