use crate::client::get_item::resolve_item;
use rustdoc_types::{Generics, ItemEnum, Type};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Implementor {
    /// Best-effort rendered path of the implementing type when it's a simple
    /// `ResolvedPath` (e.g. `"alloc::vec::Vec"`). For complex types
    /// (references, tuples, raw pointers, fn pointers, generic projections),
    /// this is `None` — inspect `for_type` for the structured form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) type_path: Option<String>,
    /// Full structured rustdoc representation of the implementing type. Use
    /// this when `type_path` is `None` or when you need generic args.
    pub(crate) for_type: Type,
    pub(crate) generics: Generics,
}

/// List types that implement the trait at `trait_path`. Returns `None` when
/// the path doesn't resolve, or when the resolved item is not a trait.
///
/// Limitations:
///   - Only implementors visible in *this* crate's rustdoc JSON are returned.
///     Foreign types implementing the trait elsewhere (e.g. impls in
///     downstream crates) are not enumerable here — that's a fundamental
///     limit of single-crate rustdoc JSON.
///   - Blanket impls (`impl<T: Bound> ThisTrait for T`) show up with the
///     blanket's `for_` (typically a generic param), so `type_path` will be
///     `None`; check `for_type` for the structured form.
pub(crate) fn list_implementors(
    docs: &rustdoc_types::Crate,
    trait_path: &[&str],
) -> Option<Vec<Implementor>> {
    let (_id, item) = resolve_item(docs, trait_path)?;
    let ItemEnum::Trait(t) = &item.inner else {
        return None;
    };

    let mut out: Vec<Implementor> = t
        .implementations
        .iter()
        .filter_map(|impl_id| {
            let impl_item = docs.index.get(impl_id)?;
            let ItemEnum::Impl(imp) = &impl_item.inner else {
                return None;
            };
            let type_path = match &imp.for_ {
                Type::ResolvedPath(p) => Some(p.path.clone()),
                _ => None,
            };
            Some(Implementor {
                type_path,
                for_type: imp.for_.clone(),
                generics: imp.generics.clone(),
            })
        })
        .collect();
    out.sort_by(|a, b| a.type_path.cmp(&b.type_path));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::docs_fixture;
    use anyhow::Result;

    #[tokio::test]
    async fn test_handler_implementors() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let path = ["axum", "handler", "Handler"];
        let implementors = list_implementors(&docs, &path).expect("Handler trait exists");

        // axum's Handler has impls (e.g. for F: FnOnce). Don't pin exact names —
        // just confirm we found some implementors.
        assert!(
            !implementors.is_empty(),
            "Handler should have at least one implementor"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_non_trait_returns_none() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;
        // Router is a struct, not a trait.
        let path = ["axum", "routing", "Router"];
        assert!(list_implementors(&docs, &path).is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_trait_returns_none() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;
        let path = ["axum", "no_such_trait"];
        assert!(list_implementors(&docs, &path).is_none());
        Ok(())
    }
}
