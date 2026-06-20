use crate::{client::changelog, context::Context, tools::render_response, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ChangelogArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version (which release's archive to fetch). Use
    /// `resolve_version` first if you only have a semver requirement.
    pub(crate) version: Version,
    /// Optional version string to extract a specific section for (best-effort:
    /// matches a markdown heading containing this string). Omit to return the
    /// full changelog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) section_version: Option<String>,
    /// Inclusive lower release bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) from_version: Option<Version>,
    /// Inclusive upper release bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) to_version: Option<Version>,
    /// Maximum releases to return. Defaults to 20.
    #[serde(default = "default_limit")]
    pub(crate) limit: usize,
    /// Return only the first paragraph of each release note.
    #[serde(default)]
    pub(crate) summary_only: bool,
    /// Maximum note characters per release. Defaults to 20,000.
    #[serde(default = "default_max_chars")]
    pub(crate) max_chars: usize,
}

fn default_limit() -> usize {
    20
}

fn default_max_chars() -> usize {
    20_000
}

#[tracing::instrument(
    name = "tool.changelog",
    skip(context),
    fields(
        krate = %args.krate,
        version = %args.version.as_ref(),
        section_version = args.section_version.as_deref(),
    ),
)]
pub(crate) async fn handle(
    context: &Context,
    args: ChangelogArgs,
) -> Result<CallToolResult, McpError> {
    render_response(
        changelog::changelog(
            context,
            &args.krate,
            args.version.as_ref(),
            changelog::ChangelogQuery {
                section_version: args.section_version.as_deref(),
                from_version: args.from_version.as_deref(),
                to_version: args.to_version.as_deref(),
                limit: args.limit,
                summary_only: args.summary_only,
                max_chars: args.max_chars,
            },
        )
        .await?,
    )
}
