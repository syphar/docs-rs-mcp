use rmcp::{ErrorData as McpError, model::CallToolResult};
use serde::Serialize;

pub(crate) mod changelog;
pub(crate) mod compare_versions;
pub(crate) mod crate_metadata;
pub(crate) mod find_examples;
pub(crate) mod get_item;
pub(crate) mod inspect_feature_flags;
pub(crate) mod list_implementors;
pub(crate) mod list_impls;
pub(crate) mod list_methods;
pub(crate) mod list_module;
pub(crate) mod manifest_dependencies;
pub(crate) mod read_source_file;
pub(crate) mod readme;
pub(crate) mod resolve_version;
pub(crate) mod search_items;
pub(crate) mod search_source;

pub(crate) fn render_response<T: Serialize>(response: T) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::structured(
        serde_json::to_value(response)
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
