# notes

## TODO:

- prefetch rustdoc json when we get resolve-version requests, including external
  dependencies?
- in-memory cache with moka? perhaps using a reduced data structure?
- in-memory cache for version-resolve? with X minutes expiration?
- on-disk sqlite representation for faster queries?
- try to locate .crate file in cargo cache? or even `/src/` cargo cache?

## mcp libs

- https://crates.io/crates/rust-mcp-sdk
- https://crates.io/crates/rmcp ( just protocol?)

- https://github.com/hyper-mcp-rs/hyper-mcp

this! likely https://lib.rs/crates/rmcp
