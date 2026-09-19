set shell := ["bash", "-cu"]
set windows-shell := ["pwsh", "-Command"]

oxfmt := "pnpm exec oxfmt"
oxlint := "pnpm exec oxlint"

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

# Lint Rust code
lint-rs:
    cargo clippy --workspace --all-targets

# Lint JavaScript code with type check
lint-js:
    {{oxlint}} --fix --fix-suggestions --fix-dangerously

# Lint code
lint: lint-rs lint-js

# Run common test
test-common:
    cargo test -p test_common -- --nocapture

# Run fixtures test
test-fixtures:
    cargo test -p test_fixtures -- --nocapture

# Run test262
test-test262:
    cargo test -p test262 -- --nocapture

# Run babel test
test-babel:
    cargo test -p test_babel -- --nocapture

# Run Eslint TypeScript test
test-eslint-typescript:
    cargo test -p test_eslint_typescript -- --nocapture

# Run test
test: test-common test-fixtures test-test262 test-babel test-eslint-typescript

# Run bench
bench:
    cargo bench -p bench -- --warm-up-time 1 --measurement-time 3

# Check code
check: fmt ls-lint typos lint test

# Publish the crate as dry-run
publish-try:
    cargo publish -p oxc_estree_codec --dry-run

# Publish the crate
publish:
    cargo publish -p oxc_estree_codec

# Clean
clean:
    cargo clean
    pnpm clean
