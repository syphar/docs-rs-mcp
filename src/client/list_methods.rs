use crate::{client::get_item::resolve_item, types::rustdoc_types::ItemKind};
use rustdoc_types::{ItemEnum, Type};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Method {
    pub(crate) name: String,
    pub(crate) kind: ItemKind,
    /// Structured rustdoc representation of the method — `ItemEnum::Function`
    /// carrying generics, decl (args + output), header (async/const/unsafe/abi).
    pub(crate) signature: ItemEnum,
    /// `Some(trait_path)` when the method comes from a trait impl
    /// (e.g. `"core::clone::Clone"`); `None` for inherent methods.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) via_trait: Option<String>,
    /// First paragraph of the method's doc comment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) summary: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) deprecated: bool,
}

/// List inherent + trait-impl methods on the type at `type_path`. Returns
/// `None` if no item resolves at that path.
///
/// Limitations:
///   - Blanket impls (`impl<T: Trait> Foo for T`) are skipped — their `for_`
///     isn't a concrete type path so we can't reliably attribute them.
///   - Default trait methods that the impl doesn't override aren't repeated
///     here; call `get_item` on the trait to see them.
///   - Only function-shaped items are returned (no associated consts/types).
pub(crate) fn list_methods(docs: &rustdoc_types::Crate, type_path: &[&str]) -> Option<Vec<Method>> {
    let (type_id, _) = resolve_item(docs, type_path)?;

    let mut methods: Vec<Method> = Vec::new();
    for item in docs.index.values() {
        let ItemEnum::Impl(imp) = &item.inner else {
            continue;
        };
        match &imp.for_ {
            Type::ResolvedPath(p) if p.id == type_id => {}
            _ => continue,
        }
        let via_trait = imp.trait_.as_ref().map(|t| t.path.clone());
        for method_id in &imp.items {
            let Some(method) = docs.index.get(method_id) else {
                continue;
            };
            if !matches!(method.inner, ItemEnum::Function(_)) {
                continue;
            }
            let Some(name) = method.name.clone() else {
                continue;
            };
            methods.push(Method {
                name,
                kind: method.inner.item_kind().into(),
                signature: method.inner.clone(),
                via_trait: via_trait.clone(),
                summary: summary_of(method),
                deprecated: method.deprecation.is_some(),
            });
        }
    }

    methods.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.via_trait.cmp(&b.via_trait))
    });
    Some(methods)
}

fn summary_of(item: &rustdoc_types::Item) -> Option<String> {
    let d = item.docs.as_deref()?.trim();
    if d.is_empty() {
        return None;
    }
    let s: String = d
        .lines()
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    (!s.is_empty()).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::docs_fixture;
    use anyhow::Result;

    #[tokio::test]
    async fn test_list_router_methods() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        let path = ["axum", "routing", "Router"];
        let methods = list_methods(&docs, &path).expect("Router exists");

        // Methods we'd expect any axum 0.x Router to have.
        let by_name = |n: &str| methods.iter().find(|m| m.name == n);
        assert!(by_name("new").is_some(), "Router::new exists");
        assert!(by_name("route").is_some(), "Router::route exists");
        assert!(by_name("nest").is_some(), "Router::nest exists");
        assert!(by_name("with_state").is_some(), "Router::with_state exists");

        // Inherent methods have no via_trait.
        assert!(by_name("new").unwrap().via_trait.is_none());

        // Sanity: at least some methods came from trait impls (e.g. Clone, Default).
        assert!(methods.iter().any(|m| m.via_trait.is_some()));

        Ok(())
    }

    #[tokio::test]
    async fn test_reexport_path_resolves() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;

        // axum::Router is a re-export; should resolve and find the same methods.
        let canon = list_methods(&docs, &["axum", "routing", "Router"]).expect("canonical");
        let rexp = list_methods(&docs, &["axum", "Router"]).expect("re-export");
        assert_eq!(canon.len(), rexp.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_type_returns_none() -> Result<()> {
        let docs = docs_fixture("axum_0.8.9.json.zst").await?;
        let path = ["axum", "no_such_type"];
        assert!(list_methods(&docs, &path).is_none());
        Ok(())
    }
}
