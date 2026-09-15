#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use oxc::span::SourceType;

    use test_helpers::corpus::{collect_files, corpus_root, run_on_big_stack};
    use test_helpers::diff::{RoundtripError, roundtrip_bytes_with};

    // Babel fixture classes; classification is plain string matching (no serde).
    enum BabelClass {
        MustParse,
        MustFailParse,
    }

    // Options inherited from ancestor `options.json` files (babel merges
    // `options.json` along the fixture directory path).
    struct BabelOptions {
        throws: bool,
        source_type: Option<String>,
    }

    fn babel_options(
        fixtures_root: &Path,
        fixture_dir: &Path,
    ) -> BabelOptions {
        let mut options: BabelOptions =
            BabelOptions { throws: false, source_type: None };

        let root_str: String =
            fixtures_root.to_string_lossy().replace('\\', "/");

        let dir_str: String = fixture_dir.to_string_lossy().replace('\\', "/");

        let relative: &str = dir_str
            .strip_prefix(root_str.as_str())
            .unwrap_or_default()
            .trim_start_matches('/');

        let mut current: PathBuf = fixtures_root.to_path_buf();

        for component in relative.split('/').filter(|part| !part.is_empty()) {
            current = current.join(component);

            let Ok(text) = fs::read_to_string(current.join("options.json"))
            else {
                continue;
            };

            if text.contains("\"throws\"") {
                options.throws = true;
            }

            if options.source_type.is_none() {
                for wanted in ["module", "script", "unambiguous", "commonjs"] {
                    if text.contains(&format!("\"sourceType\": \"{wanted}\"")) {
                        options.source_type = Some(wanted.to_owned());
                        break;
                    }
                }
            }
        }

        options
    }

    // - `output.json` (or `output.extended.json`) with a non-empty `errors`
    //   array → `MustFailParse`; an empty array means the fixture parses and
    //   must not be misclassified
    // - `options.json` containing `"throws"` (on the fixture path or an
    //   ancestor) → `MustFailParse`
    // - otherwise → `MustParse`
    //
    // `flow*` directories are filtered by the caller before classification.
    fn babel_classify(
        fixtures_root: &Path,
        fixture_dir: &Path,
    ) -> BabelClass {
        for output_name in ["output.json", "output.extended.json"] {
            let Ok(output) = fs::read_to_string(fixture_dir.join(output_name))
            else {
                continue;
            };

            if let Some(index) = output.find("\"errors\"") {
                let rest: &str = &output[index + "\"errors\"".len()..];

                let rest: &str = rest.trim_start();

                if let Some(rest) = rest.strip_prefix(':') {
                    let rest: &str = rest.trim_start();

                    if !rest.starts_with(']') {
                        return BabelClass::MustFailParse;
                    }
                }
            }
        }

        if babel_options(fixtures_root, fixture_dir).throws {
            return BabelClass::MustFailParse;
        }

        BabelClass::MustParse
    }

    // Source type refinement for babel fixtures: `options.json` `sourceType` on
    // the fixture path refines the extension-derived type, and the `jsx` /
    // `typescript` fixture-tree directories imply JSX / TypeScript syntax for
    // `.js` files (babel enables those plugins for the whole tree).
    fn babel_source_type(
        fixtures_root: &Path,
        path: &Path,
    ) -> SourceType {
        let source_type: SourceType =
            SourceType::from_path(path).unwrap_or_default();

        let dir_string: String = path.to_string_lossy().replace('\\', "/");

        let root_string: String =
            fixtures_root.to_string_lossy().replace('\\', "/");

        let jsx_dir: bool =
            dir_string.strip_prefix(root_string.as_str()).is_some_and(
                |relative| relative.split('/').any(|part| part == "jsx"),
            );

        let typescript_dir: bool =
            dir_string.strip_prefix(root_string.as_str()).is_some_and(
                |relative| relative.split('/').any(|part| part == "typescript"),
            );

        let source_type: SourceType =
            if jsx_dir { source_type.with_jsx(true) } else { source_type };

        let source_type: SourceType = if typescript_dir {
            source_type.with_typescript(true)
        } else {
            source_type
        };

        match babel_options(fixtures_root, path.parent().unwrap_or(path))
            .source_type
            .as_deref()
        {
            | Some("module") => source_type.with_module(true),
            | Some("script") => source_type.with_script(true),
            | Some("commonjs") => source_type.with_commonjs(true),
            | Some("unambiguous") => source_type.with_unambiguous(true),
            | _ => source_type,
        }
    }

    // Checked-in skip-list for ambiguous or unsupported cases. Paths are
    // fragments matched against the full fixture path and are relative to
    // `babel/packages/babel-parser/test/fixtures`. Maintained by hand; the test
    // asserts each entry still matches at least one collected case, so the list
    // cannot rot silently.
    const BABEL_SKIP_LIST: &[&str] = &[
        // `pipelineOperator` plugin (`hack` proposals) — oxc parser does not support it
        "experimental/pipeline-operator-hack",
        // `pipelineOperator` plugin (`fsharp` proposal) — oxc parser does not support it
        "experimental/pipeline-operator-fsharp",
        // `placeholders` plugin — oxc parser does not support placeholder nodes
        "placeholders",
        // `v8intrinsic` plugin — oxc parser does not support `%Name%` intrinsics
        "v8intrinsic",
        // `moduleBlocks` plugin (`module { }` blocks) — oxc parser does not support it
        "experimental/module-blocks",
        // `throwExpressions` plugin — oxc parser does not support it
        "experimental/throw-expression",
        // `doExpressions` plugin — oxc parser does not support it
        "experimental/do-expressions",
        // `asyncDoExpressions` plugin — oxc parser does not support it
        "experimental/async-do-expressions",
        // `decorators-legacy` (legacy decorators on parameters etc.) — oxc parser rejects these placements
        "experimental/decorators-legacy",
        // `discardBinding` plugin (`void` bindings) — oxc parser does not support it
        "experimental/discard-binding",
        // `functionSent` plugin (`function.sent`) — oxc parser does not support it
        "experimental/function-sent",
        // `partialApplication` plugin (`?)` optional call args) — oxc parser does not support it
        "experimental/partial-application",
        // `optionalChainingAssign` plugin (`?.=`) — oxc parser does not support it
        "experimental/optional-chaining-assign",
        // `destructuringPrivate` plugin (`#x` in patterns) — oxc parser does not support it
        "experimental/destructuring-private",
        // `bind-operator` plugin (`::`) — oxc parser does not support it
        "experimental/bind-operator",
        // `exportDefaultFrom` plugin (`export A from 'x'`) — oxc parser does not support it
        "experimental/export-extensions",
        // `async do` used as an expression; async-do-expressions plugin territory
        "comments/basic/async-do-expression",
        // `@decorator` on object literal member (decorators-legacy) via `experimental/uncategorised`
        "experimental/uncategorised/49",
        // pipeline-operator plugin referenced from a core fixture (options.json `pipelineOperator`)
        "es2020/nullish-coalescing-operator/with-pipeline",
        // module blocks (`module { }`) via es2026 explicit-resource-management tree
        "es2026/async-explicit-resource-management/valid-module-block-top-level-using-binding",
        "es2026/explicit-resource-management/valid-module-block-top-level-using-binding",
        // oxc: "Keywords cannot contain escape characters" for escaped `using` keyword
        "es2026/explicit-resource-management/valid-for-await-using-binding-escaped-of-of",
        "es2026/explicit-resource-management/valid-for-using-binding-escaped-of-of",
        // oxc parser does not support Annex B.3.1 web-compat call-expression assignment targets
        "annex-b/enabled/valid-assignment-target-type",
        "annex-b/enabled/valid-assignment-target-type-createParenthesizedExpressions",
        // babel-parser harness options (`allowAwaitOutsideFunction` etc.) — unsupported options
        "core/opts/allowAwaitOutsideFunction-true",
        "core/opts/allowNewTargetOutsideFunction-true",
        "core/opts/allowSuperOutsideMethod-true",
        "core/opts/allowUndeclaredExports",
        // jsx parse-without-conditional-plugin fixture; oxc's jsx handling differs
        "jsx/errors/_no-plugin-ts-type-param",
        "jsx/errors/_no-plugin-ts-type-param-no-flow",
        // typescript plugin implied via estree fixtures; oxc rejects method type params across a newline
        "estree/class-method/typescript-type-params",
        "estree/class-method/typescript-type-params-ranges-true",
        // oxc parser: "'abstract' modifier cannot be used with a private identifier" (babel accepts)
        "estree/class-private-method/typescript-invalid-abstract",
        // oxc semantic: duplicate declarations / overloads without implementation (babel has no semantic pass)
        "typescript/class/constructor-with-modifier-names",
        "typescript/class/declare",
        "typescript/class/members-with-modifier-names",
        "typescript/class/members-with-reserved-names",
        "typescript/class/method-no-body",
        "typescript/class/method-with-newline-without-body",
        "typescript/class/modifiers-override",
        "typescript/class/parameter-properties",
        "typescript/class/properties",
        "typescript/class/static",
        // oxc parser: "Missing initializer in const declaration" (TS accepts `const x: number;`)
        "typescript/dts/no-initializer",
        // oxc parser: rest parameter trailing comma (babel accepts)
        "typescript/dts/valid-trailing-comma-for-rest",
        // oxc parser: "A required parameter cannot follow an optional parameter" (babel accepts)
        "typescript/function/declare-pattern-parameters",
        // oxc parser: "Only ambient modules can use quoted names" (babel accepts `module 'a' {}`)
        "typescript/module-namespace/head",
        // oxc parser: `declare var` inside module namespace (babel accepts)
        "typescript/scope/namespace-declaration-var",
        "typescript/scope/namespace-declaration-var-2",
        // oxc semantic: "Ambient modules cannot be nested in other modules" (babel accepts)
        "typescript/scope/redeclaration-in-nested-module",
        // oxc parser: regex flag in JSX conditional (babel accepts `<h1>` after `==` without space)
        "typescript/tsx/assignment-in-conditional-expression",
        // oxc parser: "Unexpected token" for `<C<number>></C>` JSX instantiation expressions
        "typescript/type-arguments/tsx",
        // oxc parser: "'const' modifier can only appear on a type parameter of a function, method or class" (babel accepts)
        "typescript/types/const-type-parameters",
        // oxc semantic: "'infer' declarations are only permitted in the 'extends' clause" (babel accepts)
        "typescript/types/literal-string-4",
        // oxc semantic: "'using' declarations are not allowed at the top level of a script" (babel accepts)
        "typescript/explicit-resource-management/valid-for-using-declaration-binding-of",
        // oxc accepts code babel declares as a parse error (no semantic pass in babel):
        // Annex B disabled-function-in-block / duplicate declarations / for-in initializer
        "annex-b/disabled",
        // babel-parser option fixtures (`startIndex`/`startLine`) — metadata-only options
        "core/categorized/invalid-startindex-and-startline-specified-without-startcolumn",
        "core/categorized/startline-and-startcolumn-specified",
        "core/categorized/startline-specified",
        // babel `sourceType: commonjs` option fixtures — oxc semantic is stricter
        "core/sourcetype-commonjs",
        // babel/oxc disagree on legacy-octal-adjacent BigInt literal recovery
        "core/regression/non-octal-float-strict-mode",
        "core/scope/dupl-bind-func-module",
        "core/scope/dupl-bind-func-module-sloppy",
        "core/uncategorised/545",
        // oxc accepts `await` as an identifier in sloppy mode (babel declares an error)
        "esprima/es2015-identifier/invalid_expression_await",
        // babel declares an error for the removed import-assertions plugin; oxc accepts
        "experimental/_no-plugin/auto-accessors",
        "experimental/_removed-plugins/import-assertions",
        // oxc accepts decorators babel declares as errors (restricted placements)
        "experimental/decorator-auto-accessors/not-allowed-on-methods",
        "experimental/decorators/invalid-parenthesized-call-expression",
        "experimental/decorators/invalid-parenthesized-callexpression-createParenthesizedExpressions",
        "experimental/decorators/no-constructor-decorators",
        "experimental/decorators/plugin-conflict",
        "experimental/decorators/restricted-1",
        "experimental/decorators/restricted-2",
        "experimental/decorators/export-decorated-class-without-plugin",
        // oxc accepts `declare` on abstract/TS field decorators (babel declares an error)
        "experimental/decorators/typescript-abstract-field",
        "experimental/decorators/typescript-declare-field",
        // `invalid-no-syntaxType-option` is an option-validation fixture, not a parse fixture
        "experimental/discard-binding/invalid-no-syntaxType-option",
        // oxc accepts `export default from` identifier case (babel declares an error)
        "experimental/export-extensions/invalid-default-from-identifier",
        // oxc accepts JSX html comments / non-BMP identifiers babel declares as errors
        "jsx/errors/html-comment-module",
        "jsx/errors/_no_plugin-non-BMP-identifier",
        "jsx/errors/_no_plugin",
        "jsx/errors/_no-plugin-fragment",
        "jsx/errors/_no-plugin-jsx-expression",
        "jsx/errors/unicode-escape-in-identifier",
        // oxc accepts `declare`-less invalid class/interface bodies babel declares as errors
        "typescript/dts/invalid-class-implementation",
        "typescript/dts/invalid-class-initializer",
        // oxc accepts `export =` / `import x = require` in script mode (babel errors)
        "typescript/export/equals-in-script",
        "typescript/export/invalid-as-namespace-duplicate-identifier",
        "typescript/import/equals-in-script",
        "typescript/import/equals-require-in-script",
        // oxc accepts arrow-ambiguity forms babel declares as errors
        "typescript/arrow-function/arrow-like-in-conditional-2",
        "typescript/arrow-function/generic-tsx",
        "typescript/conditional/arrow-ambiguity",
        "typescript/conditional/arrow-like",
        "typescript/conditional/arrow-param",
        // oxc accepts TS casts/ambiguities babel declares as errors
        "typescript/cast/satisfies-const-error",
        "typescript/cast/unparenthesized-assert-and-assign",
        "typescript/cast/unparenthesized-type-assertion-and-assign",
        "typescript/disallow-jsx-ambiguity/type-assertion",
        "typescript/disallow-jsx-ambiguity/type-parameter",
        // oxc accepts decorated type arguments babel declares as errors
        "typescript/decorators/type-arguments-invalid",
        // oxc accepts interface-with-newline / module-namespace invalid bodies babel declares as errors
        "typescript/interface/new-line",
        "typescript/module-namespace/invalid-body-export-named",
        "typescript/module-namespace/invalid-declare-module-identifier",
        "typescript/module-namespace/invalid-global-redeclare-block-level-variable-in-module",
        "typescript/module-namespace/module-identifier-invalid",
        // `expectPlugin` fixtures — assert a *missing* plugin; not parseable by oxc by design
        "typescript/expect-plugin/export-interface",
        "typescript/expect-plugin/export-type-named",
        "typescript/expect-plugin/export-type",
    ];

    fn collect_babel_cases() -> Vec<PathBuf> {
        let fixtures_dir: PathBuf =
            corpus_root("babel").join("packages/babel-parser/test/fixtures");

        let mut cases: Vec<PathBuf> = Vec::new();

        cases.extend(collect_files(&fixtures_dir, "js"));
        cases.extend(collect_files(&fixtures_dir, "mjs"));
        cases.extend(collect_files(&fixtures_dir, "ts"));
        cases.extend(collect_files(&fixtures_dir, "tsx"));

        cases.retain(|path| {
            // input files only — `output.json` / `options.json` are metadata
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("input."))
        });

        cases
    }

    fn babel_display(path: &Path) -> String {
        path.to_string_lossy()
            .replace('\\', "/")
            .split("babel-parser/test/fixtures/")
            .last()
            .unwrap_or_default()
            .to_owned()
    }

    fn run_babel_roundtrip() {
        let fixtures_root: PathBuf =
            corpus_root("babel").join("packages/babel-parser/test/fixtures");

        let cases: Vec<PathBuf> = collect_babel_cases();

        if cases.is_empty() {
            panic!(
                "babel submodule is empty — run `git submodule update --init`"
            );
        }

        let mut must_parse: usize = 0;

        let mut must_fail_parse: usize = 0;

        let mut skipped: usize = 0;

        let mut matched_skips: Vec<bool> = vec![false; BABEL_SKIP_LIST.len()];

        let mut failures: Vec<String> = Vec::new();

        for path in &cases {
            // flow fixtures are out of scope entirely.
            let flow_skipped: bool = path.components().any(|component| {
                component
                    .as_os_str()
                    .to_str()
                    .is_some_and(|part| part.starts_with("flow"))
            });

            if flow_skipped {
                continue;
            }

            let display: String = babel_display(path);

            // The skip-list is a set of fixture-dir fragments; entries match at
            // path-component boundaries (`skip` followed by `/` or end) so
            // sibling dirs like `declare-field` are not caught by `declare`.
            let mut skip_hit: bool = false;

            for (index, skip) in BABEL_SKIP_LIST.iter().enumerate() {
                if display.strip_prefix(skip).is_some_and(|rest| {
                    rest.starts_with('/') || rest.is_empty()
                }) {
                    matched_skips[index] = true;

                    skip_hit = true;
                }
            }

            if skip_hit {
                skipped += 1;

                continue;
            }

            let fixture_dir: &Path =
                path.parent().expect("input file must have a parent");

            let code: String = match fs::read_to_string(path) {
                | Ok(code) => code,
                | Err(_) => continue,
            };

            let source_type: SourceType =
                babel_source_type(&fixtures_root, path);

            match babel_classify(&fixtures_root, fixture_dir) {
                | BabelClass::MustFailParse => {
                    match roundtrip_bytes_with(&display, &code, source_type) {
                    | Err(RoundtripError::ParseFailed) => {
                        must_fail_parse += 1;
                    },
                    | _ => failures.push(format!(
                        "babel declared-error fixture parsed successfully: {display}"
                    )),
                }
                },
                | BabelClass::MustParse => {
                    match roundtrip_bytes_with(&display, &code, source_type) {
                    | Ok(_) => must_parse += 1,
                    | Err(RoundtripError::ParseFailed) => failures.push(format!(
                        "babel fixture failed to parse (not declared negative): {display}"
                    )),
                    | Err(RoundtripError::Roundtrip(message)) => {
                        failures.push(message);
                    },
                }
                },
            }
        }

        // Rot guard: every skip-list entry must still match at least one
        // collected case; a renamed or removed fixture means the list is stale.
        for (entry, matched) in BABEL_SKIP_LIST.iter().zip(matched_skips.iter())
        {
            assert!(
                matched,
                "babel skip-list entry no longer matches any fixture: {entry}"
            );
        }

        // Backstop 1: the corpus must stay substantial.
        assert!(
            must_parse + must_fail_parse >= 1_000,
            "expected a substantial babel corpus: {must_parse} parsed + {must_fail_parse} negatives",
        );

        // Backstop 2: both classes must be exercised (classification drift guard).
        assert!(
            must_fail_parse >= 100,
            "expected babel declared-error fixtures: {must_fail_parse}"
        );

        // Skips are counted per fixture, and whole plugin directories (e.g. the
        // ~900 `pipeline-operator-hack` fixtures) are single entries, so the
        // guard allows fixture counts up to roughly the largest plugin tree;
        // the rot guard above pins the list to real fixture dirs.
        assert!(skipped <= 900, "babel skip-list grew too large: {skipped}");

        println!(
            "babel corpus: {must_parse} parsed, {must_fail_parse} must-fail, {skipped} skipped ({} listed)",
            BABEL_SKIP_LIST.len(),
        );

        assert!(
            failures.is_empty(),
            "{} babel failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn test_corpus_babel_roundtrip() {
        run_on_big_stack(run_babel_roundtrip);
    }
}
