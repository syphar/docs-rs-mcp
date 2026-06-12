use rmcp::schemars;

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
pub(crate) enum ItemKind {
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

impl From<ItemKind> for rustdoc_types::ItemKind {
    fn from(kind: ItemKind) -> Self {
        match kind {
            ItemKind::Module => rustdoc_types::ItemKind::Module,
            ItemKind::ExternCrate => rustdoc_types::ItemKind::ExternCrate,
            ItemKind::Use => rustdoc_types::ItemKind::Use,
            ItemKind::Struct => rustdoc_types::ItemKind::Struct,
            ItemKind::StructField => rustdoc_types::ItemKind::StructField,
            ItemKind::Union => rustdoc_types::ItemKind::Union,
            ItemKind::Enum => rustdoc_types::ItemKind::Enum,
            ItemKind::Variant => rustdoc_types::ItemKind::Variant,
            ItemKind::Function => rustdoc_types::ItemKind::Function,
            ItemKind::TypeAlias => rustdoc_types::ItemKind::TypeAlias,
            ItemKind::Constant => rustdoc_types::ItemKind::Constant,
            ItemKind::Trait => rustdoc_types::ItemKind::Trait,
            ItemKind::TraitAlias => rustdoc_types::ItemKind::TraitAlias,
            ItemKind::Impl => rustdoc_types::ItemKind::Impl,
            ItemKind::Static => rustdoc_types::ItemKind::Static,
            ItemKind::ExternType => rustdoc_types::ItemKind::ExternType,
            ItemKind::Macro => rustdoc_types::ItemKind::Macro,
            ItemKind::ProcAttribute => rustdoc_types::ItemKind::ProcAttribute,
            ItemKind::ProcDerive => rustdoc_types::ItemKind::ProcDerive,
            ItemKind::AssocConst => rustdoc_types::ItemKind::AssocConst,
            ItemKind::AssocType => rustdoc_types::ItemKind::AssocType,
            ItemKind::Primitive => rustdoc_types::ItemKind::Primitive,
            ItemKind::Keyword => rustdoc_types::ItemKind::Keyword,
            ItemKind::Attribute => rustdoc_types::ItemKind::Attribute,
        }
    }
}

impl From<rustdoc_types::ItemKind> for ItemKind {
    fn from(kind: rustdoc_types::ItemKind) -> Self {
        match kind {
            rustdoc_types::ItemKind::Module => ItemKind::Module,
            rustdoc_types::ItemKind::ExternCrate => ItemKind::ExternCrate,
            rustdoc_types::ItemKind::Use => ItemKind::Use,
            rustdoc_types::ItemKind::Struct => ItemKind::Struct,
            rustdoc_types::ItemKind::StructField => ItemKind::StructField,
            rustdoc_types::ItemKind::Union => ItemKind::Union,
            rustdoc_types::ItemKind::Enum => ItemKind::Enum,
            rustdoc_types::ItemKind::Variant => ItemKind::Variant,
            rustdoc_types::ItemKind::Function => ItemKind::Function,
            rustdoc_types::ItemKind::TypeAlias => ItemKind::TypeAlias,
            rustdoc_types::ItemKind::Constant => ItemKind::Constant,
            rustdoc_types::ItemKind::Trait => ItemKind::Trait,
            rustdoc_types::ItemKind::TraitAlias => ItemKind::TraitAlias,
            rustdoc_types::ItemKind::Impl => ItemKind::Impl,
            rustdoc_types::ItemKind::Static => ItemKind::Static,
            rustdoc_types::ItemKind::ExternType => ItemKind::ExternType,
            rustdoc_types::ItemKind::Macro => ItemKind::Macro,
            rustdoc_types::ItemKind::ProcAttribute => ItemKind::ProcAttribute,
            rustdoc_types::ItemKind::ProcDerive => ItemKind::ProcDerive,
            rustdoc_types::ItemKind::AssocConst => ItemKind::AssocConst,
            rustdoc_types::ItemKind::AssocType => ItemKind::AssocType,
            rustdoc_types::ItemKind::Primitive => ItemKind::Primitive,
            rustdoc_types::ItemKind::Keyword => ItemKind::Keyword,
            rustdoc_types::ItemKind::Attribute => ItemKind::Attribute,
        }
    }
}
