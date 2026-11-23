# Oxc ESTree Codec Crate (`oxc_estree_codec`)

The library crate. It converts an ESTree JSON string into an oxc `Program` and an oxc `Program` into an ESTree JSON string. It has no CLI, bindings, or JavaScript package. Public API and usage: [README.md](../README.md). Public behavior is documented on the public items; this file covers internals.

## Key files

| Path                                                                  | Responsibility                                                                                                                   |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                                          | Entry points. `program_to_json` delegates to oxc's ESTree serializer; `json_to_program` parses the JSON and starts the read walk |
| `src/_types/options.rs`                                               | `ProgramToJsonOptions`, `JsonToProgramOptions`, `SpanEncoding`                                                                   |
| `src/errors/read.rs`                                                  | `ReadError` variants and `Display`                                                                                               |
| `src/reader/json.rs`                                                  | JSON pre-pass, sonic-rs DOM parse, `read_js_str`, `read_source_str`                                                              |
| `src/reader/engine/context.rs`                                        | `Cx`: arena builder, `Seg` path trail, span decoding, field accessors, `nodes!` dispatch                                         |
| `src/reader/engine/program.rs`                                        | `ProgramReader`, root validation, directive splitting, function bodies                                                           |
| `src/reader/literal.rs`                                               | Literal readers, including number semantics                                                                                      |
| `src/reader/{statement,expression,pattern,declaration,jsx,ts_types}/` | Per-node readers, dispatched on the `"type"` field                                                                               |

## Read path

- `json_to_program` runs `json::parse` into a `sonic_rs::Value`, then `ProgramReader::read` walks it once and builds the AST. The DOM is dropped after the walk.
- AST nodes live in the caller's `Allocator`. Build with `AstBuilder`; allocate through `cx.box_in` or `ArenaVec`. Never allocate nodes with `std::boxed::Box`.
- A reader is `fn(&Cx<'a>, &Value) -> Result<T<'a>, ReadError>`. Dispatch on `ty_of(node)`; use the `nodes!` macro for simple type-to-constructor maps.
- Use `cx.req` / `opt` / `list` / `child` for nested nodes so the `Seg` trail keeps error paths correct.
- `Program` and `FunctionBody` store directives separately in oxc. Always split them with `split_directives_and_statements`; never read a body as one statement list.
- Use `cx.flag` / `cx.flag_or` for booleans. ESTree omits flags that are false.
- Source-type refinement is public behavior; see `JsonToProgramOptions::source_type`.

## Wire-format invariants

- Parse JSON only through `json::parse`. Its pre-pass encodes lone surrogates and literal `U+FFFD` as `U+FFFD` plus four hex digits, and the readers depend on that encoding.
- Use `read_js_str` for decoded string fields such as `Literal.value` and `TemplateElement.value.cooked`. Use `read_source_str` for source text (`raw`, regex `pattern`, directives, hashbang, JSX text, including `JSXText.value`).
- For `NumericLiteral`, parse the raw JSON text of the `value` field (`as_raw_number()`), never `raw`; `raw` supplies `number_base` and the output `raw` field. This preserves `-0`, `±Infinity`, and legacy octal values. `BigIntLiteral` uses `bigint` for its value and `raw` for its base; `NullLiteral` uses `raw` as its discriminator.
- Accept arbitrary field order and unknown extra fields. Access fields by name and ignore unread fields; the eslint-typescript corpus depends on this.
- Readers must return a typed `ReadError` for unsupported input; never panic.
- `__internal::ProgramReader` is doc-hidden. It exists for tests and benchmarks, accepts a pre-built DOM, and does not run the JSON pre-pass.

## Span invariants

- Handle both `SpanEncoding` values. JavaScript parsers usually emit UTF-16 offsets.
- Span precedence and validation are public behavior; see `JsonToProgramOptions::span_encoding`.
- `cx.span` degrades to `Span::new(0, 0)`; on error it stores the first span error, which `ProgramReader::read` returns through `take_span_error`.
- UTF-16 offsets are mapped to UTF-8 byte offsets through `Utf16Map`; validation is public behavior, see `SpanEncoding::Utf16Units`.
- Template element spans are validated in `element_span` against `value.raw`; the accepted conventions are public behavior, see `JsonToProgramOptions::source_text`.
- The write path has no span logic. `program_to_json` delegates to oxc.

## Testing

The crate has no unit tests. Tests and shared helpers live in these workspace crates:

| Path                      | Covers                                                                                                                                                                    |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tests/common`            | Reader and API behavior: roundtrip, program root validation, spans and encodings, surrogates, templates, error variants, DOM mutation through `__internal::ProgramReader` |
| `tests/helpers`           | Shared roundtrip harness (`roundtrip_bytes_with`) and corpus discovery                                                                                                    |
| `tests/fixtures`          | Checked-in TypeScript and TSX fixtures                                                                                                                                    |
| `tests/test262`           | test262 corpus, including parse-negative classification                                                                                                                   |
| `tests/babel`             | Babel parser fixtures with parse-error and skip-list classification                                                                                                       |
| `tests/eslint_typescript` | typescript-eslint fixtures with `_error_` and skip-list classification                                                                                                    |

The corpus suites run `oxc parse -> program_to_json -> json_to_program -> program_to_json` and require byte-identical output. A read-path change must keep them green.

The external corpora (test262, babel, eslint-typescript) are submodules; `tests/fixtures` is checked in. If a suite reports an empty corpus, run `git submodule update --init`.

```sh
just test-common
just test-fixtures
just test-test262
just test-babel
just test-eslint-typescript
just test
just check
```

Use `just bench` for measurements. `codec_json_to_program` is the end-to-end number; `json_parse` and `ast_read` in isolation have been misleading.

## Records

Read `records/json-reader/` before changing the parse layer. The shipped DOM-then-walk path is the fastest measured implementation on native targets; wasm SIMD is untested. A simd-json tape and a single-pass streaming reader were implemented, measured, and rejected. Do not re-propose either without new evidence.

`records/json-reader/implementation.md` holds the pipeline, number semantics, and pre-pass details. This file records the rules; the record holds the measurements and rationale.
