List types that implement a public trait within one crate's rustdoc JSON. Use for “what
implements this trait?”; use `list_impls` for the inverse question about one type.

Returns target-resolution metadata and `{implementors: [{type_path?, for_type,
generics}]}`. `type_path` is omitted for complex types; `for_type` always contains the
structured type. Results cannot include implementations defined only in downstream or
unrelated crates. Returns trait-not-found or rustdoc-loading errors and requires an
exact version.
