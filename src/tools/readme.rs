use crate::{client::readme, context::Context, tools::render_response, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ReadmeArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
    /// Return only this markdown heading section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) heading: Option<String>,
    /// Return the heading index without README body content.
    #[serde(default)]
    pub(crate) headings_only: bool,
    /// Maximum body characters to return. Defaults to 30,000.
    #[serde(default = "default_max_chars")]
    pub(crate) max_chars: usize,
}

fn default_max_chars() -> usize {
    30_000
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
    let readme = readme::readme(
        context,
        &args.krate,
        args.version.as_ref(),
        args.heading.as_deref(),
        args.headings_only,
        args.max_chars,
    )
    .await
    .map_err(|err| McpError::internal_error(err.to_string(), None))?
    .ok_or_else(|| {
        McpError::resource_not_found("no README file found in this crate's source", None)
    })?;

    render_response(readme)
}
