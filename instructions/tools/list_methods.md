List methods exposed on a specific public struct, enum, union, or primitive. Prefer this
for “what methods or operations does this type have?”; use `get_item` for full docs on
one selected method.

Returns target-resolution metadata and `{methods: [{name, kind, signature, via_trait?,
summary?, deprecated}]}` for inherent methods and methods present in concrete trait
impls. It omits blanket impl methods, unoverridden default trait methods, and associated
constants/types. Canonical and re-export paths are accepted. Returns type-not-found,
unsupported-item-kind, or rustdoc-loading errors and requires an exact version.
