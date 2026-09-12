# Oxc ESTree Codec

This is a codec between the JavaScript AST (ESTree) and the Rust AST (oxc).

## Architecture

This repository is a Rust workspace.

### Crates

| Path    | Description              |
| ------- | ------------------------ |
| `crate` | The library itself       |
| `test`  | The test for the library |

## Code Standards

### Languages

- use idiomatic Rust
- use explicit types for variables
- use explicit types for exported APIs
- prefer pure functions where practical
- prefer small, composable utilities
- avoid mutation unless it is required for correctness, performance or integration

### General

- preserve the surrounding code style
- reuse existing utilities and abstractions before introducing new ones

## Editing Rules

When modifying code:

- always ask before changing public API semantics
- if behavior changes, update tests and docs accordingly

If uncertain about intended behavior:

- ask directly, do not guess
- prefer reading tests as source of truth

## Testing Rules

- add tests when adding new behavior
- update tests when behavior changes
- keep test style consistent with nearby tests

### Integrity

- do not delete failing test to make the suite pass
- do not weaken assertions to make the suite pass
- do not change expected output without understanding why the behavior changed
- do not remove coverage because the implementation is difficult to test

If an existing test fails after a change, determine whether:

1. the implementation is incorrect, or
2. the expected behavior intentionally changed

Only update the test in the second case.

## Tooling and Workflow

The repository uses:

- Cargo
- Node.js
- pnpm
- just
- ls-lint
- typos-cli

`just` is the preferred task runner over the lower-level tools.

Before running an task, inspect the available commands:

```sh
just
```

## What NOT to Do

- invent APIs, files, modules, or behavior
- assume unsupported features exist
- violate dependency boundaries
- introduce circular dependencies
- add unnecessary dependencies
- refactor unrelated code during a focuesd change
- modify generated artifacts directly when a generation workflow exists
- migrate tooling without an explicit requirement
- intoruce a second package manager
- introduce unnecessary mutation
- commit or push Git changes unless explicitly requested
