## 0.3.0 (2026-11-25)

### Breaking

- change `json_to_program` to reject invalid `start`, `end`, and `range` values with `ReadError`
- change `json_to_program` to reject non-computed property keys that are not identifiers, static literals, or private identifiers with `ReadError`
- change `json_to_program` to require a `Program` root and reject trailing JSON
- change `JsonToProgramOptions` to require the `span_encoding` field
- change the minimum supported Rust version to `1.98.0`
- change the `oxc` version range to `~0.160.0`

### Features

- add `range` fallback to `json_to_program` when `start` and `end` are absent
- add `SpanEncoding` option to select UTF-8 or UTF-16 offset input

### Fixes

- add detection of content-only and delimiter-inclusive template element spans to `json_to_program`, including CRLF/CR raw normalization
- change `json_to_program` to reject a `this` parameter on a constructor type or construct signature with `ReadError` instead of panicking or dropping it
- change the surrogate pre-pass to distinguish a literal U+FFFD from the lone-surrogate encoding

## 0.2.2 (2026-11-05)

### Breaking

- change the `oxc` version range to `~0.157.0`

## 0.2.1 (2026-10-18)

### Breaking

- change the `oxc` version range to `~0.154.0`

## 0.2.0 (2026-10-09)

### Breaking

- change the minimum supported Rust version to `1.97.0`
- change the `oxc` version range to `~0.153.0`

## 0.1.1 (2026-10-07)

### Breaking

- change the `oxc` version range to `~0.152.0`

## 0.1.0 (2026-09-15)

### Features

- add initial release
