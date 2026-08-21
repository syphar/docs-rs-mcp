Return one rustdoc item by fully-qualified canonical or re-export path. Use for a
specific item's signature, documentation, fields, variants, deprecation status, or
doctest examples—not for discovering an unknown path.

Returns target-resolution metadata and `{item: {kind, name, path, id, inner,
deprecation, span, attrs, docs?, examples?}}`; `inner` is structured rustdoc data whose
shape depends on item kind. `verbosity="signature"` omits docs and examples;
`verbosity="full"` includes them. Re-export paths resolve to the canonical result path.
Returns item-not-found or rustdoc-loading errors. Requires an exact version; use
`search_items` first when the path is uncertain.
