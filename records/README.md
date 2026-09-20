# Records

> Check this directory **before** proposing an optimization, a dependency swap, or a design change in an area listed in the index below. If a record covers the idea, read it first — the work was already executed and measured, and re-proposing it without new evidence wastes the same effort twice.

## Purpose

Each folder in this directory is the durable record of one costly investigation: work that took significant time (spikes, implementation, benchmarks, corpus runs) and whose conclusions must outlive the code.

A record exists precisely because the cost of producing it must not be paid twice. Failures are the most valuable records: they name the approaches that were **already tried and rejected**, with the numbers that decided it.

## Conventions

- One folder per record, named after the area it covers (`<area>/`).
- Each folder pairs two docs, mirroring the roles they play:
    - `design.md` — the "why": rationale, alternatives considered, what was tried, measured outcomes, verdicts, and invariants for any retry.
    - `implementation.md` — the "how": the machinery that ships today.
- A record is written once the investigation is concluded, not while it is in progress. In-progress notes stay in scratch space and are distilled into the record afterwards.
- Small, cheap changes do not need a record. Only work whose cost of being re-derived justifies one.
- Refer to code by symbol and test name, not by file path or line number. Files move and lines drift; the code outlives both. State a file path only when it is critical to the conclusion (e.g. an architectural boundary), never as a pointer to a specific location.

## Index

| Record                                    | Status | One-line outcome                                                                                                                                                        |
| ----------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`json-reader/`](./json-reader/design.md) | Closed | simd-json tape and scalar streaming reader were implemented, measured, and rejected; sonic-rs DOM-then-walk remains the fastest measured JS→Rust path on native targets |
