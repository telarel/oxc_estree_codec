# JSON reader — replacing the sonic-rs DOM (failed experiment record)

> **Status: EXECUTED AND REJECTED.** A multi-stage campaign (2026-09-19 to 2026-09-20) replaced the sonic-rs DOM parse with a simd-json tape, then with a hand-written single-pass streaming reader. Both stages were fully implemented, passed the full corpus, and were measured — and both were **slower end-to-end** than the sonic-rs implementation they replaced. The code was reverted; this document survives so the approaches are not re-proposed.

> **Do not re-propose** a simd-json tape reader or a hand-written scalar streaming lexer for the JS→Rust path without **new evidence** — i.e. a benchmark showing the baseline below no longer holds, or a wasm-target benchmark proving the SIMD-coverage argument (see [What was not tested](#what-was-not-tested)).

The machinery that ships today is described in [implementation.md](./implementation.md).

## Motivation (why the campaign ran)

The JS→Rust path parsed ESTree JSON with sonic-rs into a fully-owned DOM (`sonic_rs::Value`), then walked it once with the hand-written reader and discarded it. The DOM build looked like the bottleneck:

| bench (7.8 KB) | time      | what it includes                          |
| -------------- | --------- | ----------------------------------------- |
| `json_parse`   | ~100.9 µs | sonic-rs DOM build                        |
| `ast_read`     | ~198.4 µs | reader walk + codegen + fresh `Allocator` |
| `roundtrip`    | ~304.1 µs | all of the above                          |

Estimated physical floor for JSON-text input at this size: ~20–30 µs. The campaign's goal: minimize codec-only cost (JSON string → typed oxc `Program`) without changing the public API (`program_to_json`, `json_to_program`, `ReadError` — all frozen).

## What was tried, and the verdict per stage

| Stage | Approach                                                                                                 | Verdict                                                                                                        |
| ----- | -------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| 1     | Replace sonic-rs DOM with simd-json 0.18.1 tape; number-semantics shim for `-0` and `1e+400`             | Implemented, corpus green, **rejected: +19.5–21% slower end-to-end**                                           |
| 2     | Number-fallback hardening (test-only): adversarial sentinel tests, numeric-field audit, wasm smoke check | Kept as tests — no perf impact                                                                                 |
| 3     | Single-pass streaming reader (hand-written scalar lexer, type-first fast path, tape fallback)            | Implemented (~90 dispatched types, zero bails on fixtures), **rejected: +20% slower than the tape** — reverted |

All numbers below: Apple M1, criterion medians, identical bench layout and fixtures, baseline re-measured immediately before each stage.

### Stage 1 — simd-json tape vs sonic baseline

| bench / fixture                            | Before (sonic) | After (tape) | Delta  |
| ------------------------------------------ | -------------- | ------------ | ------ |
| `json_parse` / react_page                  | 97.53 µs       | 129.49 µs    | +32.8% |
| `json_parse` / react_page_large            | 568.85 µs      | 833.69 µs    | +46.6% |
| `ast_read` / react_page                    | 183.68 µs      | 113.79 µs    | −38.1% |
| `ast_read` / react_page_large              | 1.1051 ms      | 679.43 µs    | −38.5% |
| `codec_json_to_program` / react_page       | 391.86 µs      | 468.26 µs    | +19.5% |
| `codec_json_to_program` / react_page_large | 2.3116 ms      | 2.8026 ms    | +21.2% |

### Stage 3 — streaming reader vs stage-1 tape (same session)

| bench / fixture                            | stage-1 tape | stage-3 stream | Delta  |
| ------------------------------------------ | ------------ | -------------- | ------ |
| `codec_json_to_program` / react_page       | 467.34 µs    | 558.42 µs      | +19.5% |
| `codec_json_to_program` / react_page_large | 2.7997 ms    | 3.3645 ms      | +20.2% |

Headline: **sonic-rs was the fastest measured implementation on native targets at every stage.**

## Why it failed (the actual lesson)

1. **The tape won the walk and lost the parse.** Eliminating the DOM made the reader-side walk ~38% faster (`ast_read`), but building the tape was ~33–47% slower than sonic-rs's DOM build (`json_parse`). The walk saving never paid for the parse loss end-to-end.
2. **simd-json's SIMD advantage does not transfer to tree-shaped, field-access-heavy JSON.** simd-json is fastest on large flat documents; ESTree is deeply nested with many tiny objects and short strings, which is where sonic-rs's approach is strongest.
3. **The tape forced costs the DOM did not have:** a mandatory input copy (`to_tape` takes `&mut [u8]` because it unescapes strings in place), pre-pass byte scans (surrogate rewrite, Infinity-sentinel rewrite), and heavier JSX/string density on the react fixtures amplified all three.
4. **Skipping the double traversal does not compensate for per-byte scalar scanning.** The stage-3 streaming reader removed the tape build/walk double pass entirely — and still lost by ~20%: a hand-written scalar lexer over ~121 KB JSON is far slower than a SIMD parse, and the reader walk overlapped with lexing less than estimated.

Net: the ~20–30 µs floor was not reached by any approach. The DOM parse cost is real, but every alternative measured was worse.

## Traps any future parser swap must know

These were each discovered the hard way during the campaign:

- **`raw` is NOT the number's wire text.** The stage-1 plan initially reconstructed `NumericLiteral` values by parsing the `raw` field — correct for decimal literals, wrong for legacy octal (`070`: value field says 56, `raw.parse::<f64>()` gives 70). Only the test262 corpus caught it. The sonic semantics are: parse the raw JSON text of the **value field** (`as_raw_number()`), never `raw`. Details in [implementation.md](./implementation.md#number-semantics-the-value-field-vs-raw).
- **`1e+400` / `-1e+400` are real traffic, not exotic edges.** oxc's serializer emits them for ±Infinity `NumericLiteral`s, so any parser that hard-errors on exponent overflow (simd-json does) needs a string-safe sentinel rewrite pre-pass — which must track string regions, because a plain backwards `is_number_context` scan is fooled by string content.
- **`-0` loses its sign in every numeric parser** that returns `u64 0`. The value-field raw-text parse is what preserves it.
- **Lone-surrogate `\ud8xx` escapes** must be rewritten to oxc's in-memory `U+FFFD` + 4 hex digits encoding before parsing, or the reader cannot distinguish decoded U+FFFD from source U+FFFD. This pre-pass must survive verbatim in any rewrite.

## What was not tested

The only untested upside of the tape path was **wasm SIMD coverage**: sonic-rs has no SIMD outside x86_64/aarch64 and is scalar on `wasm32`, while simd-json has a `simd128` path. The downstream wasm build (`wasm32-wasip1-threads`) was verified to compile with the tape, but was never benchmarked. If wasm reader performance ever matters, that is the one avenue this campaign did not rule out — and it must be proven with a wasm benchmark before accepting any native regression.

## Invariants for any future attempt

- Public API frozen: `program_to_json(&Program, ProgramToJsonOptions) -> String`, `json_to_program(&str, JsonToProgramOptions) -> Result<Program, ReadError>`, and the `ReadError` variants.
- Correctness gates, in order of what they catch:
    1. The four number/sentinel regression tests (`test_read_number_edge_cases`, `test_read_infinity_string_not_corrupted`, `test_read_infinity_number_positions`, `test_read_infinity_nested_and_negative`) — note their limits: `test_read_number_edge_cases` did **not** catch the legacy-octal trap; only test262 did.
    2. The three corpus suites (test262, babel, eslint-typescript) — eslint-typescript also covers arbitrary field order and unknown extra fields.
- `just check` (fmt, ls-lint, typos, clippy, tests) green before reporting done.
- Measure with `bench/benches/estree.rs` (`just bench`) using the `codec_json_to_program` medians — `json_parse`/`ast_read` in isolation have both been misleading (stage 1 looked like a walk win, stage 3 like a parse win; neither survived end-to-end).
