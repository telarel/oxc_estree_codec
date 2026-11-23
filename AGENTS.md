# Oxc ESTree Codec

This is a codec between the JavaScript AST (ESTree) and the Rust AST (oxc).

## Documentation Map

- `crate/AGENTS.md` — crate-local architecture, invariants, and where things live
- `records/` — durable investigations; read the index first
- `README.md` — GitHub usage overview

## Architecture

This repository is a Rust workspace.

### Crates

| Path                      | Description                             |
| ------------------------- | --------------------------------------- |
| `crate`                   | The library itself                      |
| `tests/helpers`           | Shared test helpers                     |
| `tests/common`            | Common tests for the public APIs        |
| `tests/fixtures`          | Tests for the checked-in fixtures       |
| `tests/test262`           | Tests for the test262 corpus            |
| `tests/babel`             | Tests for the babel corpus              |
| `tests/eslint_typescript` | Tests for the eslint typescript corpus  |
| `bench`                   | Benchmark for measuring the performance |

## Records

`records/` holds the costly investigations that took significant time:

- read the matching record in `records/README.md` before proposing work in a listed area
- do not re-propose executed-and-rejected approaches without new evidence
- distill a concluded investigation into a record; small changes do not need one

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

### Comments

- comments explain **why**, not **what**. The code already says what it does
- if the `why` cannot be stated briefly, the code may need refactoring, not a longer comment
- delete comments that only translate code into prose:
    - restated names
    - obvious control flow
    - per-line walkthroughs
- if a comment can be shortened to 1–2 lines without losing intent, shorten it
- keep comments only for non-obvious intent:
    - invariants
    - trade-offs
    - workarounds
    - external constraints
    - surprising behavior
    - or reasons a simpler approach was rejected
- public API doc comments state the contract only, never a step-by-step narration
- document every public struct, field, and function with at least one line stating its purpose or contract; a restated name or implementation narration is not documentation

> For this rule, "public" means reachable from the crate or package entry point; `pub(crate)`, `pub(super)`, `pub(in ...)`, and `pub` items inside private modules are private

## Editing Rules

When modifying code:

- always ask before changing public API semantics
- if uncertain about intended behavior, ask directly, do not guess
- prefer reading tests as source of truth

### Documentation

- update `crate/AGENTS.md` when a change alters internal architecture, invariants, file layout, or test commands
- update the root `README.md` only when the public API changes: adding, adjusting, or removing an exported function, type, or option
- do not update the root `README.md` for internal refactors, test-only changes, dependency bumps, or performance work
- when the public API changes, keep the `crate/src/lib.rs` rustdoc examples in sync with `README.md`
- `crate/AGENTS.md` documents internals only. Do not duplicate public API semantics there; link to `README.md` instead

### Changelog

Add user-visible changes under `## Next` in `CHANGELOG.md`. Do not add internal refactors, test-only changes.

Within each group, use these `###` subheadings in order, omitting any that have no entries:

1. `Breaking`: removals, renames, replacements, signature or contract changes, and observable default changes
2. `Features`: new entry points, options, types, hooks, exports, and capabilities
3. `Fixes`: bug fixes and behavior corrections
4. `Refactor`: user-visible restructuring that preserves behavior; internal-only refactors still do not belong in the changelog
5. `Performance`: CPU, memory, or output-size improvements

If an entry fits more than one subheading, put it under the earliest one. Start each entry with `add`, `change`, `remove`, `rename`, or `replace`. Within a subheading, sort by:

1. leading verb, alphabetically: `add`, `change`, `remove`, `rename`, `replace`
2. public API surface: entry points, then options and types, then hooks and internals
3. primary API name, case-insensitively; for entries covering several related APIs, use the first listed one

Put catch-all entries that name no single API last within their verb.

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
- refactor unrelated code during a focused change
- modify generated artifacts directly when a generation workflow exists
- write comments that restate what the code does or paraphrase function names
- leave multi-paragraph comments narrating implementation steps
- introduce unnecessary mutation
- migrate tooling without an explicit requirement
- introduce a second package manager
- commit or push Git changes unless explicitly requested
