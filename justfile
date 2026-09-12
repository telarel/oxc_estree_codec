set shell := ["bash", "-cu"]
set windows-shell := ["pwsh", "-Command"]

oxfmt := "pnpm exec oxfmt"

# Default action
_:
    just --list -u

# Install
i:
    pnpm install

# Format Rust code
fmt-rs:
    cargo fmt

# Format JavaScript code
fmt-js:
    {{oxfmt}}

# Format code
fmt: fmt-rs fmt-js

# Lint code with ls-lint
ls-lint:
    ls-lint -config ./.ls-lint.yaml

# Lint code with ls-lint
lslint: ls-lint

# Lint code with typos
typos:
    typos

# Lint code
lint:
    cargo clippy --workspace --all-targets

# Rust test
test:
    cargo test -- --nocapture

# Rust bench
bench:
    cargo bench -p bench -- --warm-up-time 1 --measurement-time 3

# Check code
check: fmt ls-lint typos lint test

# Publish the crate as dry-run
publish-try:
    cargo publish -p oxc_estree_compat --dry-run

# Publish the crate
publish:
    cargo publish -p oxc_estree_compat

# Clean
clean:
    cargo clean
    pnpm clean
