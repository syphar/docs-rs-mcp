use crate::docs_rs::get_docs_status;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};

// TODO:
// * newtype around VersionReq to implement JsonSchema?

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ResolveVersionArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub krate: String,
    /// Semver requirement (e.g. "1", "^1.5", "0.14.0") or "*". Defaults to "*".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub req: Option<String>,
}

#[derive(Clone)]
pub struct DocsServer {
    tool_router: ToolRouter<DocsServer>,
}

#[tool_router]
impl DocsServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Resolve a crate version requirement against docs.rs. Returns the concrete version and if docs.rs has documentation for that release."
    )]
    async fn resolve_version(
        &self,
        Parameters(args): Parameters<ResolveVersionArgs>,
    ) -> Result<CallToolResult, McpError> {
        let version_req: semver::VersionReq =
            args.req
                .as_deref()
                .unwrap_or("*")
                .parse()
                .map_err(|err: semver::Error| {
                    McpError::invalid_params(
                        format!("invalid semver version requirement: {}", err),
                        args.req.map(|val| serde_json::to_value(val).unwrap()),
                    )
                })?;

        let status = get_docs_status(&args.krate, &version_req)
            .await
            .map_err(|err| McpError::internal_error(err.to_string(), None))?
            .ok_or_else(|| McpError::resource_not_found("crate or version not found", None))?;

        Ok(CallToolResult::structured(
            serde_json::to_value(&status)
                .map_err(|err| McpError::internal_error(err.to_string(), None))?,
        ))
    }
}

#[tool_handler]
impl ServerHandler for DocsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "MCP server exposing Rust crate documentation from docs.rs rustdoc JSON."
                    .to_string(),
            )
    }
}
