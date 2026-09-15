#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use oxc::span::SourceType;

    use test_helpers::corpus::{collect_files, corpus_root, run_on_big_stack};
    use test_helpers::diff::{RoundtripError, roundtrip_bytes_with};

    // Annex B.3.1 web-compat call-expression assignment target; oxc AST has no
    // call-expression assignment target variant, so the upstream parser rejects
    // these at parse time.
    const TEST262_PARSER_LIMITATION: &[&str] = &[
        "annexB/language/expressions/assignmenttargettype/callexpression.js",
        "annexB/language/expressions/assignmenttargettype/callexpression-as-for-in-lhs.js",
        "annexB/language/expressions/assignmenttargettype/callexpression-as-for-of-lhs.js",
        "annexB/language/expressions/assignmenttargettype/callexpression-in-compound-assignment.js",
        "annexB/language/expressions/assignmenttargettype/callexpression-in-postfix-update.js",
        "annexB/language/expressions/assignmenttargettype/callexpression-in-prefix-update.js",
        "annexB/language/expressions/assignmenttargettype/cover-callexpression-and-asyncarrowhead.js",
    ];

    struct Test262Meta {
        parse_negative: bool,
        module: bool,
        only_strict: bool,
    }

    fn extract_test262_meta(code: &str) -> Option<Test262Meta> {
        // frontmatter is delimited by `/*---` ... `---*/`; real files place it
        // after copyright comment lines, so anchor on the first `/*---`;
        // extract with plain string matching, no YAML parser.
        //
        // parse_negative: the frontmatter body contains `negative:` and, within
        // the negative block, `phase: parse`.
        //
        // module: the `flags:` list includes `module`.
        //
        // Returns None when no frontmatter block is found.
        let start: usize = code.find("/*---")?;

        let body: &str = &code[start + "/*---".len()..];

        let end: usize = body.find("---*/")?;

        let frontmatter: &str = &body[..end];

        let parse_negative: bool = has_negative_parse_phase(frontmatter);

        let module: bool = has_module_flag(frontmatter);

        let only_strict: bool = has_flag(frontmatter, "onlyStrict");

        Some(Test262Meta { parse_negative, module, only_strict })
    }

    fn has_negative_parse_phase(frontmatter: &str) -> bool {
        let Some(negative_start) = frontmatter.find("negative:") else {
            return false;
        };

        let block: &str = &frontmatter[negative_start + "negative:".len()..];

        let block_end: Option<usize> = block
            .lines()
            .skip(1)
            .enumerate()
            .find(|(_, line)| !line.starts_with(' ') && !line.starts_with('\t'))
            .map(|(index, _)| index);

        let block_end: usize =
            block_end.map_or(block.lines().count(), |index| index + 1);

        block
            .lines()
            .take(block_end)
            .any(|line| line.trim_start().starts_with("phase: parse"))
    }

    fn has_module_flag(frontmatter: &str) -> bool {
        has_flag(frontmatter, "module")
    }

    fn has_flag(
        frontmatter: &str,
        wanted: &str,
    ) -> bool {
        for line in frontmatter.lines() {
            if let Some(rest) = line.strip_prefix("flags:") {
                let items: Vec<String> = rest
                    .trim()
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .map(str::trim)
                    .map(str::to_owned)
                    .collect();

                if items.iter().any(|item| item == wanted) {
                    return true;
                }

                continue;
            }

            let trimmed: &str = line.trim();

            if let Some(item) = trimmed.strip_prefix("- ")
                && item.trim() == wanted
            {
                return true;
            }
        }

        false
    }

    fn collect_test262_cases(subdir: &str) -> Vec<PathBuf> {
        let test_dir: PathBuf =
            corpus_root("test262").join("test").join(subdir);

        let mut cases: Vec<PathBuf> = collect_files(&test_dir, "js");

        cases.retain(|path| {
            !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("FIXTURE"))
        });

        cases
    }

    fn test262_display(path: &Path) -> String {
        path.to_string_lossy()
            .replace('\\', "/")
            .split("test262/test/")
            .last()
            .unwrap_or_default()
            .to_owned()
    }

    fn run_test262_roundtrip(subdir: &str) {
        let cases: Vec<PathBuf> = collect_test262_cases(subdir);

        if cases.is_empty() {
            panic!(
                "test262 submodule is empty — run `git submodule update --init`"
            );
        }

        let mut parse_negatives: usize = 0;

        let mut parsed: usize = 0;

        let mut parser_limited: usize = 0;

        let mut unclassified: Vec<String> = Vec::new();

        let mut failures: Vec<String> = Vec::new();

        for path in &cases {
            let file: String = match fs::read_to_string(path) {
                | Ok(code) => code,
                | Err(_) => continue,
            };

            let Some(meta) = extract_test262_meta(&file) else {
                unclassified.push(path.to_string_lossy().into_owned());
                continue;
            };

            let source_type: SourceType = if meta.module {
                SourceType::mjs()
            } else {
                SourceType::script() // script
            };

            // test262 runs `onlyStrict` tests with `"use strict";` prepended
            // (see test262/INTERPRETING.md, "Strict Mode"); parse-negatives in
            // these files only fail when the directive is present.
            let code: String = if meta.only_strict && !meta.module {
                format!("\"use strict\";\n{file}")
            } else {
                file
            };

            let display: String = test262_display(path);

            // Skipped before parsing: upstream parser limitation, not a codec bug.
            if TEST262_PARSER_LIMITATION.contains(&display.as_str()) {
                parser_limited += 1;
                continue;
            }

            match roundtrip_bytes_with(&display, &code, source_type) {
            | Ok(_) if meta.parse_negative => failures.push(format!(
                "parse-negative test262 file parsed successfully: {display}"
            )),
            | Ok(_) => parsed += 1,
            | Err(RoundtripError::ParseFailed) if meta.parse_negative => {
                parse_negatives += 1;
            }
            | Err(RoundtripError::ParseFailed) => failures.push(format!(
                "test262 file failed to parse (not declared negative): {display}"
            )),
            | Err(RoundtripError::Roundtrip(message)) => {
                failures.push(message);
            }
        }
        }

        // Backstop 1: frontmatter extraction must classify nearly every file;
        // a drift in the frontmatter format would otherwise neuter the
        // classification and every file would silently disappear.
        assert!(
            unclassified.len() * 100 <= cases.len(),
            "too many unclassified test262 files in {subdir} ({} of {}): {}",
            unclassified.len(),
            cases.len(),
            unclassified.first().map(String::as_str).unwrap_or(""),
        );

        // Backstop 2 (proportional per directory): the corpus must stay
        // substantial and parse-negative tests must stay classified. The
        // aggregate across all directories still enforces parsed >= 10_000.
        let (min_parsed, min_parse_negatives): (usize, usize) = match subdir {
            | "annexB" => (1_000, 5),
            | "built-ins" => (23_000, 150),
            | "harness" => (100, 0),
            | "intl402" => (3_000, 0),
            | "language" => (19_000, 4_000),
            | "staging" => (1_400, 5),
            | _ => unreachable!("unknown test262 subdirectory: {subdir}"),
        };

        assert!(
            parsed >= min_parsed,
            "expected a large parse-positive test262 corpus in {subdir}: {parsed}"
        );

        assert!(
            parse_negatives >= min_parse_negatives,
            "expected parse-negative tests to be classified in {subdir}: {parse_negatives}"
        );

        // Rot guard: the limitation list must stay in sync with the corpus; a
        // disappeared or renamed file means the list is stale and needs pruning.
        // All 7 listed files live in annexB, so the counter only fills there and
        // the strict check applies only to the annexB test.
        if subdir == "annexB" {
            assert!(
                parser_limited == TEST262_PARSER_LIMITATION.len(),
                "parser-limitation list out of sync with corpus (skipped {parser_limited}, listed {})",
                TEST262_PARSER_LIMITATION.len(),
            );
        }

        assert!(
            failures.is_empty(),
            "{} test262 failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    // The corpus runs ~60-85s as a single test (at/over the ~60s budget), so the
    // brief's timing guard applies: run each top-level test262/test directory as
    // its own #[test] so a failure in one directory does not re-run the others.
    #[test]
    fn test_corpus_test262_annexb() {
        run_on_big_stack(|| run_test262_roundtrip("annexB"));
    }

    #[test]
    fn test_corpus_test262_built_ins() {
        run_on_big_stack(|| run_test262_roundtrip("built-ins"));
    }

    #[test]
    fn test_corpus_test262_harness() {
        run_on_big_stack(|| run_test262_roundtrip("harness"));
    }

    #[test]
    fn test_corpus_test262_intl402() {
        run_on_big_stack(|| run_test262_roundtrip("intl402"));
    }

    #[test]
    fn test_corpus_test262_language() {
        run_on_big_stack(|| run_test262_roundtrip("language"));
    }

    #[test]
    fn test_corpus_test262_staging() {
        run_on_big_stack(|| run_test262_roundtrip("staging"));
    }
}
