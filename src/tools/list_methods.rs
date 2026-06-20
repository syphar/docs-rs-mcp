use crate::{
    client::{
        get_docs::{TargetResolution, get_docs},
        list_methods,
    },
    context::Context,
    tools::render_response,
    types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

const HOST_TARGET: &str = env!("BUILD_TARGET");

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ListMethodsArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
    /// Fully-qualified path of the type whose methods to list, including the
    /// crate name (e.g. `"axum::routing::Router"` or `"axum::Router"`).
    /// Re-export paths are resolved to the canonical type.
    pub(crate) type_path: String,
    /// Optional case-insensitive method-name filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) query: Option<String>,
    /// Return only inherent methods, excluding trait implementations.
    #[serde(default)]
    pub(crate) inherent_only: bool,
    /// Return methods from this trait path only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) trait_path: Option<String>,
    /// Whether deprecated methods should be returned. Defaults to true.
    #[serde(default = "default_true")]
    pub(crate) include_deprecated: bool,
    /// Maximum methods to return. Defaults to 50.
    #[serde(default = "default_limit")]
    pub(crate) limit: usize,
    /// Target triple. Same semantics as `search_items.target`: defaults to
    /// the host the server was compiled for; falls back to the crate's
    /// docs.rs-default target on 404.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize)]
struct ListMethodsResult {
    #[serde(flatten)]
    target: TargetResolution,
    methods: Vec<list_methods::Method>,
    total_matches: usize,
    truncated: bool,
}

#[tracing::instrument(
    name = "tool.list_methods",
    skip(context),
    fields(
        krate = %args.krate,
        version = %args.version.as_ref(),
        type_path = %args.type_path,
        query = args.query.as_deref(),
        inherent_only = args.inherent_only,
        trait_path = args.trait_path.as_deref(),
        include_deprecated = args.include_deprecated,
        limit = args.limit,
        target = args.target.as_deref(),
    ),
)]
pub(crate) async fn handle(
    context: &Context,
    args: ListMethodsArgs,
) -> Result<CallToolResult, McpError> {
    let target = args.target.as_deref().unwrap_or(HOST_TARGET);
    let docs = get_docs(context, &args.krate, args.version.as_ref(), Some(target)).await?;

    let path: Vec<&str> = args.type_path.split("::").collect();

    let mut methods = list_methods::list_methods(&docs, &path)
        .ok_or_else(|| McpError::resource_not_found("type not found at the given path", None))?;
    let query = args.query.as_ref().map(|value| value.to_lowercase());
    methods.retain(|method| {
        query
            .as_ref()
            .is_none_or(|query| method.name.to_lowercase().contains(query))
            && (!args.inherent_only || method.via_trait.is_none())
            && args
                .trait_path
                .as_ref()
                .is_none_or(|trait_path| method.via_trait.as_ref() == Some(trait_path))
            && (args.include_deprecated || !method.deprecated)
    });
    let total_matches = methods.len();
    methods.truncate(args.limit);
    let truncated = methods.len() < total_matches;

    render_response(ListMethodsResult {
        target: docs.target_resolution(),
        methods,
        total_matches,
        truncated,
    })
}
