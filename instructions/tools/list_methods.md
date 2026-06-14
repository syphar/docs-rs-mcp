Answer questions like *"what methods does X have?"*, *"list X's methods"*, *"show me the API on X"*. Returns every inherent method and every method from a trait impl on the type at `type_path`. Use this whenever the user asks about the methods, operations, or behavior of a specific struct/enum — it's faster and more focused than searching by name.

Mechanism: walks every `impl` block in the crate whose `for_` resolves to `type_path` and returns the function-shaped items inside.

`type_path` is the fully-qualified path of the type, including the crate name (e.g. `"axum::routing::Router"` or `"axum::Router"`); re-export paths are resolved to the canonical type.

Each result has:
  - `name`: method name
  - `kind`: typically `function`
  - `signature`: structured `ItemEnum::Function` (generics, decl, header)
  - `via_trait`: path of the trait (e.g. `"core::clone::Clone"`) when the method comes from a trait impl, omitted for inherent methods
  - `summary`: first paragraph of the method's doc comment
  - `deprecated`: true when the method has `#[deprecated]`

Limitations:
  - Blanket impls (`impl<T: Trait> Foo for T`) are skipped — `for_` isn't a concrete type.
  - Default trait methods that the impl doesn't override aren't repeated here. Call `get_item` on the trait if you need them.
  - Only function-shaped items are returned (no associated consts/types).

Related: `list_impls` returns the traits a type implements (no method bodies). `list_implementors` is the inverse — types that implement a given trait.

Requires an exact `version` and uses the same `target` defaulting/fallback as other tools.
