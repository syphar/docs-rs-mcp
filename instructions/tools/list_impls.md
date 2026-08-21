List public trait implementations recorded for a struct, enum, union, or primitive.
Use for “which traits does this type implement?” or Send/Sync questions; use
`list_methods` for callable methods.

Returns target-resolution metadata and `{impls: [{trait_path, generics, is_synthetic,
is_blanket}]}`, including compiler-generated auto traits and applicable blanket impls.
Canonical and re-export type paths are accepted. Returns type-not-found,
unsupported-item-kind, or rustdoc-loading errors and requires an exact version.
