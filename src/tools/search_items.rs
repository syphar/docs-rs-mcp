use crate::{client::docs::get_docs, config::Config, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use rustdoc_types::{Id, ItemKind};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SearchItemsArgs {
    /// Name of the crate on crates.io / docs.rs.
    pub(crate) krate: String,
    /// Exact crate version to load rustdoc JSON for (e.g. "1.2.3"). This is
    /// not a semver requirement — ranges like "^1.2" or "*" are not accepted.
    /// Use the `resolve_version` tool first to turn a requirement into a
    /// concrete version. To find the version currently used in a local
    /// project, run `cargo tree -p <crate>` or `cargo pkgid <crate>` in the
    /// project directory, or read it from `Cargo.lock`.
    pub(crate) version: Version,
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

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
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

impl From<ItemKind> for SearchItemKind {
    fn from(kind: ItemKind) -> Self {
        match kind {
            ItemKind::Module => SearchItemKind::Module,
            ItemKind::ExternCrate => SearchItemKind::ExternCrate,
            ItemKind::Use => SearchItemKind::Use,
            ItemKind::Struct => SearchItemKind::Struct,
            ItemKind::StructField => SearchItemKind::StructField,
            ItemKind::Union => SearchItemKind::Union,
            ItemKind::Enum => SearchItemKind::Enum,
            ItemKind::Variant => SearchItemKind::Variant,
            ItemKind::Function => SearchItemKind::Function,
            ItemKind::TypeAlias => SearchItemKind::TypeAlias,
            ItemKind::Constant => SearchItemKind::Constant,
            ItemKind::Trait => SearchItemKind::Trait,
            ItemKind::TraitAlias => SearchItemKind::TraitAlias,
            ItemKind::Impl => SearchItemKind::Impl,
            ItemKind::Static => SearchItemKind::Static,
            ItemKind::ExternType => SearchItemKind::ExternType,
            ItemKind::Macro => SearchItemKind::Macro,
            ItemKind::ProcAttribute => SearchItemKind::ProcAttribute,
            ItemKind::ProcDerive => SearchItemKind::ProcDerive,
            ItemKind::AssocConst => SearchItemKind::AssocConst,
            ItemKind::AssocType => SearchItemKind::AssocType,
            ItemKind::Primitive => SearchItemKind::Primitive,
            ItemKind::Keyword => SearchItemKind::Keyword,
            ItemKind::Attribute => SearchItemKind::Attribute,
        }
    }
}

#[derive(Debug, Serialize)]
struct SearchItemsResult {
    items: Vec<SearchItemMatch>,
}

#[derive(Debug, Serialize)]
struct SearchItemMatch {
    id: Id,
    name: String,
    path: String,
    kind: SearchItemKind,
}

pub(crate) async fn handle(
    config: &Config,
    args: SearchItemsArgs,
) -> Result<CallToolResult, McpError> {
    let kind_filter = args.kind;
    let query = args.query.to_lowercase();
    let docs = get_docs(config, &args.krate, args.version.as_ref())
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?;

    let mut matches = docs
        .index
        .values()
        .filter_map(|item| {
            let kind: SearchItemKind = item.inner.item_kind().into();
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
                id: item.id,
                name,
                path,
                kind,
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
