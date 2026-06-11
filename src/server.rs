use crate::tools::resolve_version::{self, ResolveVersionArgs};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};

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
        resolve_version::handle(args).await
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
