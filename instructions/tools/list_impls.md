Answer *"which traits does X implement?"* / *"what traits is X?"*. Returns every trait impl on the type at `type_path`, including auto-derived ones (`Send`/`Sync`/`Unpin`) and blanket impls applied to it.

Each row has:
  - `trait_path`: path of the trait being implemented (e.g. `"core::clone::Clone"`)
  - `generics`: the impl's generics (params, where-clauses)
  - `is_synthetic`: auto-derived by the compiler (typically auto-traits)
  - `is_blanket`: came from a blanket like `impl<T: Bound> Foo for T`

`type_path` accepts canonical or re-export paths; the type must resolve to a struct/enum/union/primitive (those are the kinds rustdoc records direct impls on). Use `list_methods` if you want the method list instead of the trait list.

Requires an exact `version` and reports the same target-resolution metadata as other rustdoc-backed tools.
