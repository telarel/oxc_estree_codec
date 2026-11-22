## Next

### Breaking Changes

- `JsonToProgramOptions` now requires the `span_encoding` field

### What's New

- add `SpanEncoding` option to select UTF-8 or UTF-16 offset input
- add `range` fallback when `start` and `end` are absent

### What's Changed

- raise minimum supported Rust version to `1.98.0`
- update `oxc` version range to `~0.159.0`
- detect content-only and delimiter-inclusive template element spans, including CRLF/CR raw normalization
- reject invalid `start`, `end`, and `range` values with `ReadError`

## 0.2.2 (2026-11-05)

### What's Changed

- update `oxc` version range to `~0.157.0`

## 0.2.1 (2026-10-18)

### What's Changed

- update `oxc` version range to `~0.154.0`

## 0.2.0 (2026-10-09)

### What's Changed

- raise minimum supported Rust version to `1.97.0`
- update `oxc` version range to `~0.153.0`

## 0.1.1 (2026-10-07)

### What's Changed

- update `oxc` version range to `~0.152.0`

## 0.1.0 (2026-09-15)

initial release
