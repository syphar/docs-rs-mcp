use crate::{
    context::Context,
    tools::{
        changelog::{self, ChangelogArgs},
        crate_metadata::{self, CrateMetadataArgs},
        dependency_tree::{self, DependencyTreeArgs},
        find_examples::{self, FindExamplesArgs},
        get_item::{self, GetItemArgs},
        inspect_feature_flags::{self, InspectFeatureFlagsArgs},
        list_implementors::{self, ListImplementorsArgs},
        list_impls::{self, ListImplsArgs},
        list_methods::{self, ListMethodsArgs},
        list_module::{self, ListModuleArgs},
        resolve_version::{self, ResolveVersionArgs},
        search_items::{self, SearchItemsArgs},
    },
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};

pub struct DocsServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<DocsServer>,
    config: Context,
}

#[tool_router]
impl DocsServer {
    pub fn new(config: Context) -> Self {
        Self {
            tool_router: Self::tool_router(),
            config,
        }
    }

    #[doc = include_str!("../instructions/tools/resolve_version.md")]
    #[tool]
    async fn resolve_version(
        &self,
        Parameters(args): Parameters<ResolveVersionArgs>,
    ) -> Result<CallToolResult, McpError> {
        resolve_version::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/search_items.md")]
    #[tool]
    async fn search_items(
        &self,
        Parameters(args): Parameters<SearchItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        search_items::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/list_module.md")]
    #[tool]
    async fn list_module(
        &self,
        Parameters(args): Parameters<ListModuleArgs>,
    ) -> Result<CallToolResult, McpError> {
        list_module::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/get_item.md")]
    #[tool]
    async fn get_item(
        &self,
        Parameters(args): Parameters<GetItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        get_item::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/list_methods.md")]
    #[tool]
    async fn list_methods(
        &self,
        Parameters(args): Parameters<ListMethodsArgs>,
    ) -> Result<CallToolResult, McpError> {
        list_methods::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/list_impls.md")]
    #[tool]
    async fn list_impls(
        &self,
        Parameters(args): Parameters<ListImplsArgs>,
    ) -> Result<CallToolResult, McpError> {
        list_impls::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/list_implementors.md")]
    #[tool]
    async fn list_implementors(
        &self,
        Parameters(args): Parameters<ListImplementorsArgs>,
    ) -> Result<CallToolResult, McpError> {
        list_implementors::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/inspect_feature_flags.md")]
    #[tool]
    async fn inspect_feature_flags(
        &self,
        Parameters(args): Parameters<InspectFeatureFlagsArgs>,
    ) -> Result<CallToolResult, McpError> {
        inspect_feature_flags::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/crate_metadata.md")]
    #[tool]
    async fn crate_metadata(
        &self,
        Parameters(args): Parameters<CrateMetadataArgs>,
    ) -> Result<CallToolResult, McpError> {
        crate_metadata::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/dependency_tree.md")]
    #[tool]
    async fn dependency_tree(
        &self,
        Parameters(args): Parameters<DependencyTreeArgs>,
    ) -> Result<CallToolResult, McpError> {
        dependency_tree::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/changelog.md")]
    #[tool]
    async fn changelog(
        &self,
        Parameters(args): Parameters<ChangelogArgs>,
    ) -> Result<CallToolResult, McpError> {
        changelog::handle(&self.config, args).await
    }

    #[doc = include_str!("../instructions/tools/find_examples.md")]
    #[tool]
    async fn find_examples(
        &self,
        Parameters(args): Parameters<FindExamplesArgs>,
    ) -> Result<CallToolResult, McpError> {
        find_examples::handle(&self.config, args).await
    }
}

#[tool_handler]
impl ServerHandler for DocsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(include_str!("../instructions/server.md").to_string())
    }
}
