public-api KRATE VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    curl -sSfL "https://static.crates.io/crates/{{KRATE}}/{{KRATE}}-{{VERSION}}.crate" \
      | tar -xz -C "$tmp"
    cargo public-api -sss --manifest-path "$tmp/{{KRATE}}-{{VERSION}}/Cargo.toml"
