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
    /// Semver requirement. A fully-qualified bare version such as "1.2.3"
    /// is treated as an exact match. Partial versions and explicit
    /// requirements use Cargo semantics. Examples: "1.2", "^1.5", "~1.2",
    /// ">=1.2, <1.5", "*". Defaults to "*" (latest).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{client::status::Status, test_utils::test_env};
    use anyhow::Result;

    #[tokio::test]
    async fn resolver_cache_is_scoped_by_crate_name() -> Result<()> {
        let mut env = test_env().await?;
        let axum = Status {
            doc_status: true,
            version: semver::Version::new(0, 8, 9),
        };
        let tokio = Status {
            doc_status: true,
            version: semver::Version::new(1, 45, 1),
        };

        let _axum_mock = env
            .server
            .mock("GET", "/crate/axum/*/status.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&axum)?)
            .create();
        let _tokio_mock = env
            .server
            .mock("GET", "/crate/tokio/*/status.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&tokio)?)
            .create();

        let axum_result = handle(
            env.context(),
            ResolveVersionArgs {
                krate: "axum".into(),
                req: VersionReq::default(),
            },
        )
        .await?;
        let tokio_result = handle(
            env.context(),
            ResolveVersionArgs {
                krate: "tokio".into(),
                req: VersionReq::default(),
            },
        )
        .await?;

        assert_eq!(
            axum_result.structured_content,
            Some(serde_json::to_value(axum)?)
        );
        assert_eq!(
            tokio_result.structured_content,
            Some(serde_json::to_value(tokio)?)
        );
        Ok(())
    }
}
