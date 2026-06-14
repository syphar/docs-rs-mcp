Answer *"what implements this trait?"* / *"which types are X?"*. The inverse of `list_impls`. Returns every type that implements the trait at `trait_path` *within this crate's rustdoc JSON*.

Each row has:
  - `type_path`: rendered path of the implementing type when it's a simple resolved path (e.g. `"alloc::vec::Vec"`); omitted for complex types like `&T`, tuples, or function pointers — inspect `for_type` then
  - `for_type`: full structured rustdoc representation of the implementing type
  - `generics`: the impl's generics

Limitation: rustdoc JSON is single-crate. Implementations in *other* crates (e.g. a downstream crate implementing this trait on its own type) are not visible here. To find those you'd have to search those crates explicitly.

Requires an exact `version` and uses the same `target` defaulting/fallback as other tools.
