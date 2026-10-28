# Development tasks. `just` lists them; `just ci` runs what CI runs.

msrv := `sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml`

# List the recipes
default:
    @just --list

# Everything CI checks, except the other operating systems
ci: fmt-check lint test doc deny unused-deps typos msrv

# Format Rust and TOML
fmt:
    cargo fmt --all
    taplo fmt

# Check formatting
fmt-check:
    cargo fmt --all --check
    taplo fmt --check

# Clippy with warnings as errors, for every feature combination
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo hack clippy --workspace --all-targets --feature-powerset -- -D warnings

# Unit, property, CLI and doc tests (no hardware needed)
test:
    cargo nextest run --workspace --all-features
    cargo test --workspace --all-features --doc

# Round trip every reversible setting on connected speakers
test-hardware:
    cargo test -p klipsch-lumina --test hardware -- --ignored --test-threads 1

# API docs, failing on any warning
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

# Open the API docs
doc-open: doc
    cargo doc --workspace --all-features --no-deps --open

# Advisories, licences, duplicate and unknown-source dependencies
deny:
    cargo deny check

# Dependencies declared but not used
unused-deps:
    cargo machete

# Spelling
typos:
    typos

# Build with the minimum supported Rust version
msrv:
    rustup toolchain install {{msrv}} --profile minimal
    cargo +{{msrv}} check --workspace --all-targets --all-features

# Coverage report in target/llvm-cov/html
coverage:
    cargo llvm-cov --workspace --all-features --html
    @echo "open target/llvm-cov/html/index.html"
