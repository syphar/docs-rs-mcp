Quick orientation for a crate: name, version, description, repository, homepage, license, documentation URL, MSRV (`rust-version`), edition, authors, keywords, categories. Reads the `[package]` section from the crate's `Cargo.toml` on crates.io.

Use this when the user asks *"what is this crate?"* / *"who authored it?"* / *"what's the MSRV?"* / *"what license?"*. Cargo manifest authors are not a reliable list of current crates.io owners or maintainers.

NOT for *"what version is X?"* / *"latest X"* / *"X 0.8"* — this tool needs a fully-specified exact version (e.g. `"1.2.3"`) and won't accept `"*"`, `"0.8"`, `"^1.0"`, or any range. Call `resolve_version` first to turn any non-exact requirement into a concrete version, then pass that here.
