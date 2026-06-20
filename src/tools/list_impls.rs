use crate::{
    client::{
        get_docs::{TargetResolution, get_docs},
        list_impls,
    },
    context::Context,
    tools::render_response,
    types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

const HOST_TARGET: &str = env!("BUILD_TARGET");

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ListImplsArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
    /// Fully-qualified path of the type whose impls to list, including the
    /// crate name (e.g. `"axum::routing::Router"` or `"axum::Router"`).
    /// Re-export paths are resolved to the canonical type.
    pub(crate) type_path: String,
    /// Target triple. Same semantics as `search_items.target`: defaults to
    /// the host the server was compiled for; falls back to the crate's
    /// docs.rs-default target on 404.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListImplsResult {
    #[serde(flatten)]
    target: TargetResolution,
    impls: Vec<list_impls::Impl>,
}

#[tracing::instrument(
    name = "tool.list_impls",
    skip(context),
    fields(
        krate = %args.krate,
        version = %args.version.as_ref(),
        type_path = %args.type_path,
        target = args.target.as_deref(),
    ),
)]
pub(crate) async fn handle(
    context: &Context,
    args: ListImplsArgs,
) -> Result<CallToolResult, McpError> {
    let target = args.target.as_deref().unwrap_or(HOST_TARGET);
    let docs = get_docs(context, &args.krate, args.version.as_ref(), Some(target)).await?;

    let path: Vec<_> = args.type_path.split("::").collect();

    let impls = list_impls::list_impls(&docs, &path).ok_or_else(|| {
        McpError::resource_not_found(
            "no type (struct/enum/union/primitive) found at the given path",
            None,
        )
    })?;

    render_response(ListImplsResult {
        target: docs.target_resolution(),
        impls,
    })
}
