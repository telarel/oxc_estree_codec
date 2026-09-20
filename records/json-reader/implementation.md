# JSON reader — implementation

> The rationale behind this — including the two parser-swap attempts that were measured and rejected — lives in [design.md](./design.md).

## Summary

The JS→Rust path parses ESTree JSON with sonic-rs into a fully-owned DOM (`sonic_rs::Value`), then walks it exactly once with a hand-written reader that builds an oxc `Program` in a caller-provided `Allocator`. The DOM is discarded after the walk. All JSON-representation knowledge is confined to one module (`crate/src/reader/json.rs`); the ~6k-line reader never sees the JSON library.

## Pipeline

`json_to_program` does exactly three things:

1. `crate::reader::json::parse(json)` — pre-processes the input, then parses it into a `sonic_rs::Value` DOM.
2. `ProgramReader::new(options.allocator)` — creates the reader state.
3. `reader.read(&value, source_type, source_text)` — single recursive walk over the DOM, building the oxc AST bottom-up.

## Module map

```
crate/src/reader/
  json.rs              pre-passes (lone surrogates) + sonic-rs DOM parse;
                       the ONLY module that imports sonic_rs
  literal.rs           literals: string/numeric/regexp/bigint (number path
                       is the subtlest code in the reader — see below)
  engine/
    context.rs         Cx: arena allocation, spans, error trails (Seg),
                       field accessors (req/opt/list), the
                       req_readers!/opt_readers!/list_readers! macros
    program.rs         ProgramReader entry, directive splitting,
                       function-body reading
  statement/           statements + control flow
  expression/          expressions, operators, template literals
  pattern/             binding patterns and assignment targets
  declaration/         classes, enums, functions, imports/exports,
                       interfaces, modules
  jsx/                 JSX elements, attributes, containers
  ts_types/            TypeScript type names, signatures, types
```

Per-node-kind readers live in the subdirectories and are dispatched on the `"type"` field via `ty_of`.

## Wire-format contract

Producers on the wire:

- oxc's `to_estree_json` (Rust side): type-first field order, stable field order, emits `raw` on literals.
- `JSON.stringify` in downstream JS (V8 preserves insertion order of an object that originated from `JSON.parse` of oxc output → type-first in practice).

The reader must also keep working on input that violates these conventions: the eslint-typescript corpus contains trees with arbitrary field order and extra unknown fields (the reader ignores unknown fields and accesses fields by name, so it already does).

## Number semantics — the value field vs `raw`

The subtlest part of the reader. For a `NumericLiteral`, the wire carries two different numbers:

- `"value"` — the canonical numeric value; its raw JSON **text** is what the reader must parse to reconstruct the exact f64.
- `"raw"` — the **source text** of the literal (e.g. `070`, `1e999`). It is NOT the value's wire text. They agree for decimal literals and diverge for other radices.

The reader therefore parses the value field's raw JSON text: `node.get("value").as_raw_number().as_str().parse::<f64>()`. sonic-rs exposes the raw text of a number via `as_raw_number()`, which is what makes this exact: `-0` keeps its sign, `1e+400` parses to `Infinity`, and full precision is preserved — identical to `JSON.parse` semantics.

**The legacy-octal trap (caught by test262, not by unit tests):** parsing `raw` instead of the value field turns `070` (value `56`) into `70`, because `"070".parse::<f64>()` parses decimal. `raw` is correct only for `number_base` detection, where it is genuinely the source text. Any parser swap that loses access to the value field's raw text must repair `-0` (sign) and `±Infinity` (sentinel) cases from wire text — and must use `raw` only where the value is genuinely unrepresentable on the tape/DOM, never as the general value source.

The sentinel cases, all real traffic (emitted by oxc's serializer for non-finite values, `oxc_estree`):

| source   | value field on wire | `raw`    | reader must produce |
| -------- | ------------------- | -------- | ------------------- |
| `-0`     | `-0`                | `-0`     | `-0.0`              |
| `1e999`  | `1e+400`            | `1e999`  | `f64::INFINITY`     |
| `-1e999` | `-1e+400`           | `-1e999` | `f64::NEG_INFINITY` |

## Pre-pass: lone-surrogate escapes

`encode_lone_surrogate_escapes` (in the pre-pass module) rewrites JSON `\ud8xx` escapes with no low half into oxc's in-memory encoding: `U+FFFD` followed by 4 lowercase hex digits. Reason: JSON decoders (sonic-rs included) decode unpaired escapes to U+FFFD, and the reader must distinguish a decoded lone surrogate from a U+FFFD that was literally in the source. Paired surrogates (`\ud834\udd1e`) decode correctly everywhere and are untouched. This pre-pass runs before parsing and must be preserved verbatim in any rewrite of the parse layer.

## Error model

Reader failures are `ReadError`: typed variants for the common cases (`NodeUnsupported`, `FieldMissing`, `FieldInvalid`, `OperatorUnsupported`, `ImportPhaseUnsupported`, …) plus a message-carrying variant for JSON parse errors. During the walk, `Cx` maintains a path trail of `Seg` (field name or array index) segments (`Cx::push`/`Cx::pop`/`Cx::child`, `Cx::path_string`) so every error reports where in the JSON tree it occurred.

## Public API surface

- `program_to_json(&Program, ProgramToJsonOptions) -> String` — thin wrapper over oxc's `to_estree_json`.
- `json_to_program(&str, JsonToProgramOptions) -> Result<Program, ReadError>`.
- `ProgramToJsonOptions` / `JsonToProgramOptions`; `JsonToProgramOptions` carries the `Allocator`, `source_type`, and `source_text`.
- `__internal::ProgramReader` (doc-hidden) — used by this repo's own tests to read pre-built `sonic_rs::Value` trees (mutation tests).
