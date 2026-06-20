use crate::{
    client::status::get_docs_status, context::Context, tools::render_response,
    types::semver::VersionReq,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use std::sync::Arc;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ResolveVersionArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Cargo-style semver requirement. Bare versions are caret requirements:
    /// "1.2.3" means ">=1.2.3, <2.0.0" (compatible), not an exact match.
    /// Use "=1.2.3" for an exact version. Other examples: "1", "^1.5",
    /// "~1.2", ">=1.2, <1.5", "*". Defaults to "*" (latest).
    #[serde(default)]
    pub(crate) req: VersionReq,
}

#[tracing::instrument(
    name = "tool.resolve_version",
    skip(context),
    fields(krate = %args.krate, req = %args.req.as_ref()),
)]
pub(crate) async fn handle(
    context: &Context,
    args: ResolveVersionArgs,
) -> Result<CallToolResult, McpError> {
    let status = context
        .resolver_cache
        .entry((args.krate.clone(), args.req.clone().into()))
        .or_try_insert_with::<_, anyhow::Error>(async move {
            Ok(Arc::new(
                get_docs_status(context, &args.krate, args.req.as_ref()).await?,
            ))
        })
        .await;

    if let Some(status) = status
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .into_value()
        .as_ref()
    {
        Ok(render_response(status)?)
    } else {
        Err(McpError::resource_not_found(
            "crate or version not found",
            None,
        ))
    }
}
