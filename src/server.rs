use crate::{
    context::Context,
    tools::{
        changelog::{self, ChangelogArgs},
        crate_metadata::{self, CrateMetadataArgs},
        dependency_tree::{self, DependencyTreeArgs},
        find_examples::{self, FindExamplesArgs},
        get_item::{self, GetItemArgs},
        inspect_feature_flags::{self, InspectFeatureFlagsArgs},
        list_implementors::{self, ListImplementorsArgs},
        list_impls::{self, ListImplsArgs},
        list_methods::{self, ListMethodsArgs},
        list_module::{self, ListModuleArgs},
        resolve_version::{self, ResolveVersionArgs},
        search_items::{self, SearchItemsArgs},
    },
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};

pub struct DocsServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<DocsServer>,
    config: Context,
}

#[tool_router]
impl DocsServer {
    pub fn new(config: Context) -> Self {
        Self {
            tool_router: Self::tool_router(),
            config,
        }
    }

    #[tool(
        description = "\
Answer questions like *\"what's the latest version of X?\"* / *\"latest 0.8.x of axum\"* / \
*\"what 1.0-compatible version is out?\"* / *\"does docs.rs have version Y of X?\"* / \
*\"is X published?\"*. Resolves a crate name + a semver requirement to a *concrete* published \
version, and tells you whether docs.rs built documentation for it.

ALWAYS call this first whenever the user gives anything other than a fully-specified \
`MAJOR.MINOR.PATCH` (e.g. `\"1.2.3\"`) — every other tool in this server requires an exact \
version string. In particular, `\"*\"`, `\"0.8\"`, `\"^1.0\"`, `\"~1.2\"`, `\">=1.0, <2\"` are \
NOT valid `version` arguments anywhere else; pass them as `req` here first.

The `req` argument is a semver requirement. Note: this server diverges from \
Cargo on one specific case — a bare *fully-qualified* `MAJOR.MINOR.PATCH` is \
treated as an EXACT match (matching docs.rs URL semantics), not Cargo's caret \
default. Everything else is parsed as a normal Cargo requirement.

  - `\"*\"`, `\"latest\"`, `\"newest\"` → latest published version overall (case-insensitive).
  - `\"1.2.3\"` or `\"=1.2.3\"` → exactly version 1.2.3.
  - `\"1.2\"` or `\"^1.2\"` → latest 1.x ≥ 1.2 (caret).
  - `\"~1.2.3\"` → latest 1.2.x.
  - `\">=1.0, <2\"` → latest version in that range.

Result is `{ version, doc_status }`. `doc_status = true` means docs.rs has built rustdoc JSON for \
that release — required for `search_items`, `list_module`, `get_item`, etc."
    )]
    async fn resolve_version(
        &self,
        Parameters(args): Parameters<ResolveVersionArgs>,
    ) -> Result<CallToolResult, McpError> {
        resolve_version::handle(&self.config, args).await
    }

    #[tool(description = "\
Search rustdoc items for a crate version by name or path, optionally filtering by item kind.
Requires an exact version — call `resolve_version` first if you only have a semver requirement.

Defaults to fetching docs for the *host* target — the triple this server was compiled for, \
which is almost always the user's own machine. So a Windows user gets Windows docs, a Linux \
user gets Linux docs, etc.

Override via the `target` arg when the user's project targets something different from their \
host (check `Cargo.toml [build] target`, `.cargo/config.toml`, or anything they've said about \
deployment). The common case is macOS/Windows dev → Linux server deploy: pass \
`target: \"x86_64-unknown-linux-gnu\"` then. Using the wrong target hides items gated on \
`#[cfg(target_os = ...)]` and can surface items that won't compile on the real target.

If docs.rs has no build for the requested `target` (most crates only opt into one target), \
the server falls back to the *crate's* default target — whichever target the crate author \
marked as default in their docs.rs metadata. For most crates that's \
`x86_64-unknown-linux-gnu`, but a Windows-centric crate like `windows-sys` might default to \
a Windows triple. The fallback is silent and assumes the crate's API is the same across \
targets; cfg-gated items may then be missing or extra — verify against the user's actual \
target if precision matters.

Each result has: `id`, `name`, `path` (import path the user writes, e.g. `axum::Router`), \
`kind` (`struct`, `trait`, `function`, ...), `aliases` (values declared via \
`#[doc(alias = \"...\")]`; the query also matches against these), and optionally `reexport`.

Re-exports (`pub use ...`) are first-class. The same item may appear at multiple paths: \
its canonical definition and every `pub use` that re-exports it. Each path is independently \
importable. There is no automatic dedup — if you want one entry per item, dedup on `id`.

When `reexport` is set, the match was reached via a `pub use` chain rather than the item's \
canonical home:
  - `reexport.source_crate`: crate the original definition lives in (omitted when same crate as the searched one).
  - `reexport.source_version`: version of that source crate, parsed from its docs.rs URL when available.

To follow a re-export to its canonical definition, call `search_items` again with \
`krate = reexport.source_crate` and `version = reexport.source_version` (use `resolve_version` \
first if the version is missing). Repeat if that result is also a re-export.

The response also includes `unexpanded_external_globs`: glob re-exports (`pub use foo::*`) \
that pull from external crates. The server does not expand them — for each entry, follow up \
with another `search_items` call against `source_crate` and `source_version`. Items found \
there at path `P` are also reachable in the searched crate at `<prefix>::<P>`.

Caveats:
  - Some paths surfaced here may go through private modules (importable name is the re-export, \
    not the canonical path).
  - `#[doc(hidden)]` items may appear — not part of the stable API.

Follow-ups: for a struct/enum/trait result, call `list_methods` to see its methods or \
`get_item` to read its signature, docs, and examples. For a module result, call `list_module` \
to enumerate its children.")]
    async fn search_items(
        &self,
        Parameters(args): Parameters<SearchItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        search_items::handle(&self.config, args).await
    }

    #[tool(description = "\
List the direct children of a module in a crate. Returns one row per child with `name`, \
`kind`, `summary` (first paragraph of the doc comment, if any), `deprecated`, and optionally \
`reexport` (when the child is a `pub use` of something else).

Requires an exact version — call `resolve_version` first if you only have a semver requirement.
Defaults the `target` to the host the server was compiled for, with the same fallback to the \
crate's docs.rs-default target on 404 as `search_items`.

Use this to browse a crate's surface one module at a time. `path` is the fully-qualified \
module path including the crate name (e.g. `\"axum::extract\"`); omit it to list the crate \
root. Non-glob `pub use` re-exports appear as their own row, with `reexport.source_crate` \
identifying where the underlying item lives. Glob re-exports (`pub use foo::*`) of *external* \
crates are reported separately via `unexpanded_external_globs` — follow up by calling \
`list_module` (or `search_items`) on the source crate.")]
    async fn list_module(
        &self,
        Parameters(args): Parameters<ListModuleArgs>,
    ) -> Result<CallToolResult, McpError> {
        list_module::handle(&self.config, args).await
    }

    #[tool(description = "\
Return the full record for a single item by its fully-qualified path. Requires an exact \
version — call `resolve_version` first if you only have a semver requirement.

`path` accepts either canonical or re-export paths (e.g. `\"axum::Router\"` resolves to its \
canonical `\"axum::routing::Router\"`); the `path` field on the result is always the \
canonical path. Returns the same `target` defaulting and fallback semantics as \
`search_items` / `list_module`.

The result includes:
  - `kind`, `name`, `path`, `id`
  - `inner`: structured rustdoc info (signature, generics, where-clauses, \
    fields/variants/function decl, etc.); shape varies by `kind`
  - `deprecation`, `span`, `attrs`

`verbosity` controls how much detail to return (default `\"full\"`):
  - `\"signature\"`: signature only (the structured `inner`). Cheap.
  - `\"full\"`: signature + raw `docs` + `examples` (Rust fenced code blocks extracted from \
    the doc string). Blocks tagged with non-Rust languages are skipped; rustdoc attributes \
    like `ignore`, `no_run`, `compile_fail`, `editionXXXX` are treated as Rust. Hidden \
    doctest lines (starting with `#`) are kept verbatim — strip or substitute as needed.

Follow-ups: when the item is a struct/enum/union, call `list_methods` on the same path to \
see its methods. When it's a module, call `list_module` to enumerate its children. When it's \
a trait, `list_methods` on a concrete type that implements it shows which methods are \
inherited; `get_item` on the trait itself shows the trait's required/default methods inside \
`inner`.")]
    async fn get_item(
        &self,
        Parameters(args): Parameters<GetItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        get_item::handle(&self.config, args).await
    }

    #[tool(description = "\
Answer questions like *\"what methods does X have?\"*, *\"list X's methods\"*, *\"show me the \
API on X\"*. Returns every inherent method and every method from a trait impl on the type at \
`type_path`. Use this whenever the user asks about the methods, operations, or behavior of a \
specific struct/enum — it's faster and more focused than searching by name.

Mechanism: walks every `impl` block in the crate whose `for_` resolves to `type_path` and \
returns the function-shaped items inside.

`type_path` is the fully-qualified path of the type, including the crate name (e.g. \
`\"axum::routing::Router\"` or `\"axum::Router\"`); re-export paths are resolved to the \
canonical type.

Each result has:
  - `name`: method name
  - `kind`: typically `function`
  - `signature`: structured `ItemEnum::Function` (generics, decl, header)
  - `via_trait`: path of the trait (e.g. `\"core::clone::Clone\"`) when the method comes from \
    a trait impl, omitted for inherent methods
  - `summary`: first paragraph of the method's doc comment
  - `deprecated`: true when the method has `#[deprecated]`

Limitations:
  - Blanket impls (`impl<T: Trait> Foo for T`) are skipped — `for_` isn't a concrete type.
  - Default trait methods that the impl doesn't override aren't repeated here. Call \
    `get_item` on the trait if you need them.
  - Only function-shaped items are returned (no associated consts/types).

Related: `list_impls` returns the traits a type implements (no method bodies). \
`list_implementors` is the inverse — types that implement a given trait.

Requires an exact `version` and uses the same `target` defaulting/fallback as other tools.")]
    async fn list_methods(
        &self,
        Parameters(args): Parameters<ListMethodsArgs>,
    ) -> Result<CallToolResult, McpError> {
        list_methods::handle(&self.config, args).await
    }

    #[tool(description = "\
Answer *\"which traits does X implement?\"* / *\"what traits is X?\"*. Returns every trait \
impl on the type at `type_path`, including auto-derived ones (`Send`/`Sync`/`Unpin`) and \
blanket impls applied to it.

Each row has:
  - `trait_path`: path of the trait being implemented (e.g. `\"core::clone::Clone\"`)
  - `generics`: the impl's generics (params, where-clauses)
  - `is_synthetic`: auto-derived by the compiler (typically auto-traits)
  - `is_blanket`: came from a blanket like `impl<T: Bound> Foo for T`

`type_path` accepts canonical or re-export paths; the type must resolve to a \
struct/enum/union/primitive (those are the kinds rustdoc records direct impls on). Use \
`list_methods` if you want the method list instead of the trait list.

Requires an exact `version` and uses the same `target` defaulting/fallback as other tools.")]
    async fn list_impls(
        &self,
        Parameters(args): Parameters<ListImplsArgs>,
    ) -> Result<CallToolResult, McpError> {
        list_impls::handle(&self.config, args).await
    }

    #[tool(description = "\
Answer *\"what implements this trait?\"* / *\"which types are X?\"*. The inverse of \
`list_impls`. Returns every type that implements the trait at `trait_path` *within this \
crate's rustdoc JSON*.

Each row has:
  - `type_path`: rendered path of the implementing type when it's a simple resolved path \
    (e.g. `\"alloc::vec::Vec\"`); omitted for complex types like `&T`, tuples, or function \
    pointers — inspect `for_type` then
  - `for_type`: full structured rustdoc representation of the implementing type
  - `generics`: the impl's generics

Limitation: rustdoc JSON is single-crate. Implementations in *other* crates (e.g. a \
downstream crate implementing this trait on its own type) are not visible here. To find \
those you'd have to search those crates explicitly.

Requires an exact `version` and uses the same `target` defaulting/fallback as other tools.")]
    async fn list_implementors(
        &self,
        Parameters(args): Parameters<ListImplementorsArgs>,
    ) -> Result<CallToolResult, McpError> {
        list_implementors::handle(&self.config, args).await
    }

    #[tool(description = "\
Answer *\"how do I enable feature X?\"*, *\"what features does this crate have?\"*, *\"does \
this crate need `tokio/macros`?\"*. Reads the `[features]` section from the crate's \
`Cargo.toml`.

Each row is `{name, enables, default}`:
  - `enables`: the verbatim entries Cargo will activate (other features, `dep:foo` syntax for \
    optional deps, `foo/bar` for transitive feature activation).
  - `default`: true when this feature is in the crate's `default` feature set.

Requires an exact `version` (the published Cargo.toml on crates.io for that release).")]
    async fn inspect_feature_flags(
        &self,
        Parameters(args): Parameters<InspectFeatureFlagsArgs>,
    ) -> Result<CallToolResult, McpError> {
        inspect_feature_flags::handle(&self.config, args).await
    }

    #[tool(description = "\
Quick orientation for a crate: name, version, description, repository, homepage, license, \
documentation URL, MSRV (`rust-version`), edition, authors, keywords, categories. Reads the \
`[package]` section from the crate's `Cargo.toml` on crates.io.

Use this when the user asks *\"what is this crate?\"* / *\"who maintains it?\"* / *\"what's \
the MSRV?\"* / *\"what license?\"*.

NOT for *\"what version is X?\"* / *\"latest X\"* / *\"X 0.8\"* — this tool needs a fully-\
specified exact version (e.g. `\"1.2.3\"`) and won't accept `\"*\"`, `\"0.8\"`, `\"^1.0\"`, \
or any range. Call `resolve_version` first to turn any non-exact requirement into a \
concrete version, then pass that here.")]
    async fn crate_metadata(
        &self,
        Parameters(args): Parameters<CrateMetadataArgs>,
    ) -> Result<CallToolResult, McpError> {
        crate_metadata::handle(&self.config, args).await
    }

    #[tool(description = "\
Direct dependencies of a crate (one level, not transitive). Useful for *\"what does this \
crate pull in?\"* / *\"is X already a dep of Y?\"*. Reads `[dependencies]`, \
`[dev-dependencies]`, `[build-dependencies]`, and target-gated sections from `Cargo.toml`.

Each row: `{name, kind: \"normal\"|\"dev\"|\"build\", req, optional, default_features, \
features, target?}`. `target` is set when the dep is gated under \
`[target.'cfg(...)'.dependencies]`. The name `dependency_tree` is aspirational — this is a \
flat one-level list, not a recursive tree.")]
    async fn dependency_tree(
        &self,
        Parameters(args): Parameters<DependencyTreeArgs>,
    ) -> Result<CallToolResult, McpError> {
        dependency_tree::handle(&self.config, args).await
    }

    #[tool(description = "\
Return the crate's changelog content. Tries common filenames (`CHANGELOG.md`, `CHANGES.md`, \
`HISTORY.md`, `NEWS.md`, and their extensionless variants) and returns the first that exists.

When `section_version` is provided, returns only the heading section that mentions that \
version string — best-effort heuristic against markdown heading syntax. Omit it for the \
full changelog.

Use this for *\"what changed in 1.4?\"* / *\"any breaking changes between 0.7 and 0.8?\"* / \
*\"is there a CHANGELOG?\"*.")]
    async fn changelog(
        &self,
        Parameters(args): Parameters<ChangelogArgs>,
    ) -> Result<CallToolResult, McpError> {
        changelog::handle(&self.config, args).await
    }

    #[tool(description = "\
List `.rs` files under the crate's `examples/` directory. Many crates publish runnable \
examples here — real working code is often more useful than doctest fragments.

Each row: `{path, name, content?}`. `name` is the file stem (or directory name for \
multi-file examples). `content` is only included when `include_content=true`.

Use this for *\"show me how to use this crate\"* / *\"are there full working examples?\"* — \
preferable to extracting doctests when the crate ships proper examples.")]
    async fn find_examples(
        &self,
        Parameters(args): Parameters<FindExamplesArgs>,
    ) -> Result<CallToolResult, McpError> {
        find_examples::handle(&self.config, args).await
    }
}

#[tool_handler]
impl ServerHandler for DocsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "MCP server exposing Rust crate documentation from docs.rs rustdoc JSON.\n\
                 \n\
                 Typical flow:\n\
                 1. `resolve_version(krate, req)` to turn a semver requirement (or `*`) \
                    into a concrete version. All other tools take an exact version.\n\
                 2. Discover paths with `search_items` (search by name/path/alias, \
                    filtered by kind) or `list_module` (enumerate one module's children).\n\
                 3. Drill in:\n   \
                    - `get_item(path)` — full record for one item: signature, docs, \
                      examples.\n   \
                    - `list_methods(type_path)` — methods on a struct/enum/union.\n   \
                    - `list_impls(type_path)` — traits a type implements.\n   \
                    - `list_implementors(trait_path)` — types that implement a trait \
                      (single-crate only).\n   \
                    - `list_module(path)` — recurse into a submodule.\n\
                 \n\
                 Match the tool to the question:\n   \
                 - any version question or non-exact version (\"latest of X\", \"X 0.8\", \
                 \"X ^1.0\", \"does X 1.2 exist\", \"is docs.rs ready\") → `resolve_version` \
                 (pass the requirement as `req`; use `req=\"*\"` for latest). \
                 NEVER pass anything other than a fully-specified `MAJOR.MINOR.PATCH` as \
                 `version` to any other tool — `\"*\"`, `\"0.8\"`, `\"^1.0\"`, `\"~1.2\"`, \
                 ranges, etc. are invalid; resolve them through `resolve_version` first.\n   \
                 - \"what methods does X have\" / \"how do I use X\" → `list_methods`.\n   \
                 - \"which traits does X implement\" / \"is X Send/Sync/...\" → `list_impls`.\n   \
                 - \"what implements this trait\" / \"which types are X\" → `list_implementors`.\n   \
                 - \"what's in module X\" / \"what does X re-export\" → `list_module`.\n   \
                 - \"find X\" / \"is there a Y in this crate\" → `search_items`.\n   \
                 - \"show me X's signature/docs/examples\" → `get_item`.\n   \
                 - \"what features does X have\" / \"how do I enable X\" → `inspect_feature_flags`.\n   \
                 - \"what is this crate\" / \"MSRV / license / repo\" → `crate_metadata`.\n   \
                 - \"what does X depend on\" → `dependency_tree`.\n   \
                 - \"what changed in X\" / \"any breaking changes\" → `changelog`.\n   \
                 - \"show me a working example\" → `find_examples` first, fall back to `get_item` doctests.\n\
                 \n\
                 The `target` arg on the drill-in tools defaults to the host the server was \
                 compiled for (usually the user's machine). Override when the user's \
                 *project* targets something different (e.g. macOS dev → Linux deploy)."
                    .to_string(),
            )
    }
}
