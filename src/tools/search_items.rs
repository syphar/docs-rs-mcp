use crate::{config::Config, rustdoc_json::get_docs};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use rustdoc_types::ItemKind;
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SearchItemsArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Concrete crate version to load rustdoc JSON for.
    pub(crate) version: String,
    /// Search text matched against item names and paths.
    pub(crate) query: String,
    /// Optional item kind filter, e.g. "struct", "enum", "trait", "function", "module".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<SearchItemKind>,
    /// Maximum number of matches to return. Defaults to 20.
    #[serde(default = "default_limit")]
    pub(crate) limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename_all = "snake_case")]
pub(crate) enum SearchItemKind {
    Module,
    ExternCrate,
    Use,
    Struct,
    StructField,
    Union,
    Enum,
    Variant,
    Function,
    TypeAlias,
    Constant,
    Trait,
    TraitAlias,
    Impl,
    Static,
    ExternType,
    Macro,
    ProcAttribute,
    ProcDerive,
    AssocConst,
    AssocType,
    Primitive,
    Keyword,
    Attribute,
}

impl From<SearchItemKind> for ItemKind {
    fn from(kind: SearchItemKind) -> Self {
        match kind {
            SearchItemKind::Module => ItemKind::Module,
            SearchItemKind::ExternCrate => ItemKind::ExternCrate,
            SearchItemKind::Use => ItemKind::Use,
            SearchItemKind::Struct => ItemKind::Struct,
            SearchItemKind::StructField => ItemKind::StructField,
            SearchItemKind::Union => ItemKind::Union,
            SearchItemKind::Enum => ItemKind::Enum,
            SearchItemKind::Variant => ItemKind::Variant,
            SearchItemKind::Function => ItemKind::Function,
            SearchItemKind::TypeAlias => ItemKind::TypeAlias,
            SearchItemKind::Constant => ItemKind::Constant,
            SearchItemKind::Trait => ItemKind::Trait,
            SearchItemKind::TraitAlias => ItemKind::TraitAlias,
            SearchItemKind::Impl => ItemKind::Impl,
            SearchItemKind::Static => ItemKind::Static,
            SearchItemKind::ExternType => ItemKind::ExternType,
            SearchItemKind::Macro => ItemKind::Macro,
            SearchItemKind::ProcAttribute => ItemKind::ProcAttribute,
            SearchItemKind::ProcDerive => ItemKind::ProcDerive,
            SearchItemKind::AssocConst => ItemKind::AssocConst,
            SearchItemKind::AssocType => ItemKind::AssocType,
            SearchItemKind::Primitive => ItemKind::Primitive,
            SearchItemKind::Keyword => ItemKind::Keyword,
            SearchItemKind::Attribute => ItemKind::Attribute,
        }
    }
}

#[derive(Debug, Serialize)]
struct SearchItemsResult {
    items: Vec<SearchItemMatch>,
}

#[derive(Debug, Serialize)]
struct SearchItemMatch {
    id: String,
    name: String,
    path: String,
    kind: String,
}

pub(crate) async fn handle(
    config: &Config,
    args: SearchItemsArgs,
) -> Result<CallToolResult, McpError> {
    let version = args.version.parse().map_err(|err: semver::Error| {
        McpError::invalid_params(
            format!("invalid semver version: {}", err),
            Some(serde_json::json!({ "version": args.version })),
        )
    })?;

    let kind_filter = args.kind.map(ItemKind::from);
    let query = args.query.to_lowercase();
    let docs = get_docs(config, &args.krate, &version)
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?;

    let mut matches = docs
        .index
        .values()
        .filter_map(|item| {
            let kind = item.inner.item_kind();
            if kind_filter.is_some_and(|filter| filter != kind) {
                return None;
            }

            let path = docs
                .paths
                .get(&item.id)
                .map(|summary| summary.path.join("::"))
                .or_else(|| item.name.clone())?;
            let name = item.name.clone().unwrap_or_default();
            let haystack = format!("{name} {path}").to_lowercase();

            haystack.contains(&query).then_some(SearchItemMatch {
                id: item.id.0.to_string(),
                name,
                path,
                kind: serialize_item_kind(kind).ok()?,
            })
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    matches.truncate(args.limit);

    Ok(CallToolResult::structured(
        serde_json::to_value(SearchItemsResult { items: matches })
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}

fn serialize_item_kind(kind: ItemKind) -> Result<String, serde_json::Error> {
    Ok(serde_json::to_value(kind)?
        .as_str()
        .expect("ItemKind should serialize as a string")
        .to_string())
}
