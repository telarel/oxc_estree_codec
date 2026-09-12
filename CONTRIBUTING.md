[< Back](./README.md)

# Contributing to Oxc ESTree Codec

Thanks for your interest in contributing!

This is a guideline for contributing to Oxc ESTree Codec.

## Before the Contribution

Please install the following dependencies:

| Dependencies                                   | Description                            |
| ---------------------------------------------- | -------------------------------------- |
| [Rust](https://rust-lang.org)                  | Programming language                   |
| [Node.js](https://nodejs.org)                  | JavaScript runtime                     |
| [pnpm](https://pnpm.io)                        | Programming language                   |
| [just](https://just.systems)                   | Command runner                         |
| [ls-lint](https://ls-lint.org)                 | Linting tool for directories and files |
| [typos-cli](https://github.com/crate-ci/typos) | Spell checker                          |

## Commands

Check the available commands with the following command:

```sh
just
```

## Committing

When committing changes to the code, use the following prefixes:

- `chore`: updates in dependencies/tools
- `build`: changes to the build system
- `fix`: fixes a bug
- `feat`: adds a new feature
- `refactor`: other code changes
- `perf`: performance improvements
- `security`: security related changes
- `style`: style changes
- `test`: adding or updating tests
- `docs`: documentation only changes
- `ci`: CI configuration updates
- `release`: new version release

For example:

```
feat: add xxx feature
docs: fix typos
```
