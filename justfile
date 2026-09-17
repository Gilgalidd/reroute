# Developer shortcuts. Install `just` (https://github.com/casey/just) or run
# the commands by hand; they are all plain cargo / npm invocations.

set shell := ["bash", "-cu"]

default: check

# Everything CI runs, in one go.
check: fmt-check clippy test deny web-check web-test

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

deny:
    cargo deny check

audit:
    cargo audit

web-install:
    cd apps/desktop && npm ci

web-check:
    cd apps/desktop && npm run check

web-test:
    cd apps/desktop && npm test

# Run the app with hot reload (opens the settings window).
dev:
    cd apps/desktop && npm run tauri dev

# Run the app on a URL, as the OS would.
dev-url url="https://example.com/":
    cd apps/desktop && npm run tauri dev -- -- -- "{{url}}"

# Build installers for the current platform into target/release/bundle.
bundle:
    cd apps/desktop && npm run tauri build

coverage:
    cargo llvm-cov --workspace --lcov --output-path lcov.info
