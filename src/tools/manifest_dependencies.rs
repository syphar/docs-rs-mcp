use crate::{
    client::manifest_dependencies, context::Context, tools::render_response, types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ManifestDependenciesArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
    /// Filter by normal, dev, or build dependency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<manifest_dependencies::DependencyKind>,
    /// Return only optional dependencies.
    #[serde(default)]
    pub(crate) optional_only: bool,
}

#[derive(Debug, Serialize)]
struct ManifestDependenciesResult {
    dependencies: Vec<manifest_dependencies::Dependency>,
}

#[tracing::instrument(
    name = "tool.manifest_dependencies",
    skip(context),
    fields(krate = %args.krate, version = %args.version.as_ref()),
)]
pub(crate) async fn handle(
    context: &Context,
    args: ManifestDependenciesArgs,
) -> Result<CallToolResult, McpError> {
    let mut dependencies =
        manifest_dependencies::manifest_dependencies(context, &args.krate, args.version.as_ref())
            .await?;
    dependencies.retain(|dependency| {
        args.kind.is_none_or(|kind| dependency.kind == kind)
            && (!args.optional_only || dependency.optional)
    });

    render_response(ManifestDependenciesResult { dependencies })
}
