use crate::client::get_item::resolve_item;
use rustdoc_types::{Generics, Id, ItemEnum};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Impl {
    /// Path of the trait being implemented (e.g. `"core::clone::Clone"`).
    pub(crate) trait_path: String,
    pub(crate) generics: Generics,
    /// Auto-derived by the compiler (typically auto-trait impls like `Send`,
    /// `Sync`, `Unpin`).
    pub(crate) is_synthetic: bool,
    /// This impl came from a blanket like `impl<T: Bound> Foo for T` rather
    /// than a direct `impl Foo for ThisType`.
    pub(crate) is_blanket: bool,
}

/// List the traits implemented by the type at `type_path`. Returns `None`
/// when no type resolves at the path, or when the resolved item is not a
/// struct/enum/union/primitive (those are the only kinds rustdoc records
/// impls for directly).
pub(crate) fn list_impls(docs: &rustdoc_types::Crate, type_path: &[&str]) -> Option<Vec<Impl>> {
    let (_id, item) = resolve_item(docs, type_path)?;
    let impl_ids = type_impls(item)?;

    let mut out: Vec<Impl> = impl_ids
        .iter()
        .filter_map(|impl_id| {
            let impl_item = docs.index.get(impl_id)?;
            let ItemEnum::Impl(imp) = &impl_item.inner else {
                return None;
            };
            let trait_ = imp.trait_.as_ref()?;
            Some(Impl {
                trait_path: trait_.path.clone(),
                generics: imp.generics.clone(),
                is_synthetic: imp.is_synthetic,
                is_blanket: imp.blanket_impl.is_some(),
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a.trait_path
            .cmp(&b.trait_path)
            .then_with(|| a.is_blanket.cmp(&b.is_blanket))
    });
    Some(out)
}

fn type_impls(item: &rustdoc_types::Item) -> Option<&[Id]> {
    match &item.inner {
        ItemEnum::Struct(s) => Some(&s.impls),
        ItemEnum::Enum(e) => Some(&e.impls),
        ItemEnum::Union(u) => Some(&u.impls),
        ItemEnum::Primitive(p) => Some(&p.impls),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::docs_fixture;
    use anyhow::Result;

    #[tokio::test]
    async fn test_router_impls() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let path = ["axum", "routing", "Router"];
        let impls = list_impls(&docs, &path).expect("Router exists");

        let trait_names: Vec<&str> = impls.iter().map(|i| i.trait_path.as_str()).collect();
        // Spot-checks against well-known impls. Use contains rather than full
        // pin because rustdoc's exact path strings can vary by edition.
        assert!(
            trait_names.iter().any(|p| p.ends_with("Clone")),
            "Router should implement Clone"
        );
        assert!(
            trait_names.iter().any(|p| p.ends_with("Debug")),
            "Router should implement Debug"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_reexport_path_resolves() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let canon = list_impls(&docs, &["axum", "routing", "Router"]).expect("canonical");
        let rexp = list_impls(&docs, &["axum", "Router"]).expect("re-export");
        assert_eq!(canon.len(), rexp.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_type_returns_none() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;
        let path = ["axum", "no_such_type"];
        assert!(list_impls(&docs, &path).is_none());
        Ok(())
    }
}
