#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};

    use oxc::span::SourceType;

    use test_helpers::corpus::{collect_files, corpus_root};
    use test_helpers::diff::{RoundtripError, roundtrip_bytes_with};

    // typescript-eslint ast-spec fixtures are laid out as
    // `packages/ast-spec/src/**/fixtures/**/fixture.ts(x)` in this tag (not
    // `packages/ast-spec/tests/fixtures`). Directories with an `_error_`
    // component contain intentionally invalid code: their parse or semantic
    // failure is expected, and the harness reports both as `ParseFailed`.
    fn collect_typescript_eslint_cases() -> Vec<PathBuf> {
        let parser_fixtures: PathBuf = corpus_root("eslint_typescript")
            .join("packages/parser/tests/fixtures");

        let ast_spec_fixtures: PathBuf =
            corpus_root("eslint_typescript").join("packages/ast-spec/src");

        let mut cases: Vec<PathBuf> = Vec::new();

        cases.extend(collect_files(&parser_fixtures, "ts"));
        cases.extend(collect_files(&parser_fixtures, "tsx"));

        cases.extend(collect_files(&ast_spec_fixtures, "ts"));
        cases.extend(collect_files(&ast_spec_fixtures, "tsx"));

        cases
    }

    // An ast-spec `_error_` fixture is a genuine parse-error case only when the
    // upstream `TSESTree - Error` snapshot records an error; many `_error_` dirs
    // exist because *Babel* fails while TSESTree parses the file fine ("NO ERROR"
    // in the snapshot), and those must parse here too.
    fn is_error_fixture(path: &Path) -> bool {
        if !path.components().any(|component| {
            component.as_os_str().to_str().is_some_and(|part| part == "_error_")
        }) {
            return false;
        }

        let snapshot: PathBuf = path
            .parent()
            .map(|dir| dir.join("snapshots/1-TSESTree-Error.shot"))
            .unwrap_or_default();

        match fs::read_to_string(&snapshot) {
            | Ok(snapshot) => {
                snapshot.lines().any(|line| line.trim() == "TSError")
            },
            | Err(_) => true,
        }
    }

    // Matches `fragment` at path-component boundaries: the character before and
    // after the match must be `/` (or the string edge). Prevents sibling dirs
    // like `declare-field` from matching a `declare` fragment.
    fn contains_path_component(
        display: &str,
        fragment: &str,
    ) -> bool {
        let mut from: usize = 0;

        while let Some(start) = display[from..].find(fragment) {
            let start: usize = from + start;

            let end: usize = start + fragment.len();

            let before: bool =
                start == 0 || display.as_bytes()[start - 1] == b'/';

            let after: bool =
                end == display.len() || display.as_bytes()[end] == b'/';

            if before && after {
                return true;
            }

            from = start + 1;
        }

        false
    }

    // `_error_` fixtures where TypeScript reports an error but the oxc parser
    // accepts the code. Entries are path fragments under
    // `typescript-eslint/packages/ast-spec/src` (or, for the one parser fixture,
    // `typescript-eslint/packages/parser/tests/fixtures`) matched at
    // path-component boundaries. Maintained by hand; the test asserts each
    // entry still matches at least one collected case.
    const TS_SKIP_LIST: &[&str] = &[
        // TS errors ("Identifier expected"); oxc accepts a declare function with no id
        "declaration/TSDeclareFunction/fixtures/_error_/missing-id-and-not-exported",
        // TS errors ("'async' modifier cannot be used in an ambient context"); oxc accepts `declare async function`
        "declaration/TSDeclareFunction/fixtures/_error_/async",
        // TS errors ("'extends' list cannot be empty"); oxc accepts `extends implements` as a heritage name
        "declaration/TSInterfaceDeclaration/fixtures/_error_/extends-implements",
        // TS errors ("'source' is not a valid meta-property"); oxc supports the stage-3 import source phase
        "expression/MetaProperty/fixtures/_error_/import-source",
        // TS errors ("Abstract property cannot have an initializer"); oxc accepts the initializer
        "element/AccessorProperty/fixtures/_error_/modifier-abstract-accessor-with-value",
        // TS errors ("Computed property names are not allowed in enums"); oxc allows string computed names
        "element/TSEnumMember/fixtures/_error_/computed-string-name",
        // TS errors ("Computed property names are not allowed in enums"); oxc allows string computed names
        "legacy-fixtures/basics/fixtures/_error_/export-named-enum-computed-string",
        // oxc parser: "Only ambient modules can use quoted names" (TS accepts `module 'a' {}`)
        "declaration/TSModuleDeclaration/fixtures/module-id-literal",
        // oxc semantic: "'using' declarations are not allowed at the top level of a script" (TS accepts)
        "declaration/VariableDeclaration/fixtures/await-using-multiple-declarations",
        // oxc semantic: "'using' declarations are not allowed at the top level of a script" (TS accepts)
        "declaration/VariableDeclaration/fixtures/await-using-with-value",
        // oxc semantic: "'using' declarations are not allowed at the top level of a script" (TS accepts)
        "declaration/VariableDeclaration/fixtures/using-id-init",
        // oxc semantic: "'using' declarations are not allowed at the top level of a script" (TS accepts)
        "declaration/VariableDeclaration/fixtures/using-id-type-init",
        // oxc semantic: "'using' declarations are not allowed at the top level of a script" (TS accepts)
        "declaration/VariableDeclaration/fixtures/using-multiple-declarations",
        // oxc parser: "'accessor' modifier cannot be used with 'declare' modifier" (TS accepts)
        "element/AccessorProperty/fixtures/modifier-declare",
        // oxc parser: "'accessor' modifier cannot be used with 'readonly' modifier" (TS accepts)
        "element/AccessorProperty/fixtures/modifier-readonly",
        // oxc parser: "Type annotation cannot appear on a constructor declaration" (TS accepts)
        "legacy-fixtures/basics/fixtures/class-with-constructor-and-return-type",
        // oxc parser: "Identifier expected. 'await' is a reserved word" (TS accepts `await` as a binding)
        "legacy-fixtures/basics/fixtures/keyword-variables",
        // oxc parser: "A 'set' accessor cannot have a return type annotation" (TS accepts)
        "legacy-fixtures/basics/fixtures/object-with-typed-methods",
        // oxc parser: "'declare' modifier cannot be used in an already ambient context" (TS accepts)
        "legacy-fixtures/namespaces-and-modules/fixtures/global-module-declaration",
        // oxc parser: "Only ambient modules can use quoted names" (TS accepts)
        "legacy-fixtures/namespaces-and-modules/fixtures/module-with-default-exports",
        // oxc parser: "An index signature must have a type annotation" (TS accepts, ast-spec errorRecovery fixture)
        "legacy-fixtures/types/fixtures/index-signature-without-type",
        // oxc parser: "'const' modifier can only appear on a type parameter of a function, method or class" (TS accepts)
        "special/TSTypeParameter/fixtures/interface-const-in-modifier-multiple",
        // oxc parser: "'const' modifier can only appear on a type parameter of a function, method or class" (TS accepts)
        "special/TSTypeParameter/fixtures/interface-const-modifier",
        // oxc parser: "'const' modifier can only appear on a type parameter of a function, method or class" (TS accepts)
        "special/TSTypeParameter/fixtures/interface-const-modifier-extends",
        // oxc parser: "'const' modifier can only appear on a type parameter of a function, method or class" (TS accepts)
        "special/TSTypeParameter/fixtures/interface-const-modifier-multiple",
        // oxc parser: "'const' modifier can only appear on a type parameter of a function, method or class" (TS accepts)
        "special/TSTypeParameter/fixtures/interface-in-const-modifier-multiple",
        // oxc parser: "'const' modifier can only appear on a type parameter of a function, method or class" (TS accepts)
        "special/TSTypeParameter/fixtures/method-const-modifiers",
        // oxc parser: "Missing initializer in const declaration" (TS accepts)
        "scope-analysis/expression-type-parameters.ts",
        // oxc parser errors like Babel ("Missing initializer in destructuring declaration"); TS-ESTree snapshot says NO ERROR
        "declaration/VariableDeclaration/fixtures/_error_/const-destructure-no-init",
        // oxc parser errors like Babel ("Missing initializer in destructuring declaration"); TS-ESTree snapshot says NO ERROR
        "declaration/VariableDeclaration/fixtures/_error_/const-destructure-type-no-init",
        // oxc parser errors like Babel ("Missing initializer in const declaration"); TS-ESTree snapshot says NO ERROR
        "declaration/VariableDeclaration/fixtures/_error_/const-id-no-init",
        // oxc parser errors like Babel ("Missing initializer in const declaration"); TS-ESTree snapshot says NO ERROR
        "declaration/VariableDeclaration/fixtures/_error_/const-id-type-no-init",
        // oxc parser errors like Babel ("Missing initializer in destructuring declaration"); TS-ESTree snapshot says NO ERROR
        "declaration/VariableDeclaration/fixtures/_error_/let-destructure-no-init",
        // oxc parser errors like Babel ("Missing initializer in destructuring declaration"); TS-ESTree snapshot says NO ERROR
        "declaration/VariableDeclaration/fixtures/_error_/let-destructure-type-no-init",
        // oxc parser errors like Babel ("Missing initializer in var declaration"); TS-ESTree snapshot says NO ERROR
        "declaration/VariableDeclaration/fixtures/_error_/var-destructure-no-init",
        // oxc parser errors like Babel ("Missing initializer in destructuring declaration"); TS-ESTree snapshot says NO ERROR
        "declaration/VariableDeclaration/fixtures/_error_/var-destructure-type-no-init",
        // oxc parser: "Unexpected new.target expression" (TS accepts in arrow function body)
        "legacy-fixtures/basics/fixtures/_error_/new-target-in-arrow-function-body",
        // oxc parser: "Await expression cannot be used" (TS accepts `await` without async in a .ts fixture)
        "legacy-fixtures/basics/fixtures/_error_/await-without-async-function",
        // oxc parser: accessibility on private identifier field; TS-ESTree snapshot says NO ERROR
        "legacy-fixtures/basics/fixtures/_error_/class-private-identifier-field-with-accessibility-error",
        // oxc parser: type parameters on constructor; TS-ESTree snapshot says NO ERROR
        "legacy-fixtures/basics/fixtures/_error_/class-with-constructor-and-type-parameters",
        // oxc parser: computed constructor; TS-ESTree snapshot says NO ERROR
        "legacy-fixtures/basics/fixtures/_error_/class-with-two-methods-computed-constructor",
        // oxc parser: `as const` on every literal; TS-ESTree snapshot says NO ERROR
        "legacy-fixtures/basics/fixtures/_error_/const-assertions",
        // oxc parser: index signature parameters; TS-ESTree snapshot says NO ERROR
        "legacy-fixtures/errorRecovery/fixtures/_error_/index-signature-parameters",
        // oxc parser: optional index signature; TS-ESTree snapshot says NO ERROR
        "legacy-fixtures/errorRecovery/fixtures/_error_/interface-with-optional-index-signature",
    ];

    // Per-run record of which skip-list entries matched; reset at test start.
    thread_local! {
        static TS_SKIP_MATCHED: RefCell<Vec<bool>> =
            const { RefCell::new(Vec::new()) };
    }

    #[test]
    fn test_corpus_eslint_typescript_roundtrip() {
        let cases: Vec<PathBuf> = collect_typescript_eslint_cases();

        if cases.is_empty() {
            panic!(
                "eslint_typescript submodule is empty — run `git submodule update --init`"
            );
        }

        TS_SKIP_MATCHED.with_borrow_mut(|matched| {
            *matched = vec![false; TS_SKIP_LIST.len()]
        });

        let mut parsed: usize = 0;

        let mut must_fail: usize = 0;

        let mut skipped: usize = 0;

        let mut failures: Vec<String> = Vec::new();

        for path in &cases {
            let display: String = path.to_string_lossy().replace('\\', "/");

            // The skip-list is a set of fixture-dir fragments matched against
            // the full path at component boundaries so sibling dirs cannot
            // shadow each other.
            let mut skip_hit: bool = false;

            for (index, skip) in TS_SKIP_LIST.iter().enumerate() {
                if contains_path_component(&display, skip) {
                    TS_SKIP_MATCHED
                        .with(|slot| slot.borrow_mut()[index] = true);

                    skip_hit = true;
                }
            }

            if skip_hit {
                skipped += 1;

                continue;
            }

            let code: String = match fs::read_to_string(path) {
                | Ok(code) => code,
                | Err(_) => continue,
            };

            let display: &str = &display;

            let source_type: SourceType =
                SourceType::from_path(path).unwrap_or_default();

            match roundtrip_bytes_with(display, &code, source_type) {
            | Ok(_) if is_error_fixture(path) => failures.push(format!(
                "eslint_typescript _error_ fixture parsed successfully: {display}"
            )),
            | Ok(_) => parsed += 1,
            | Err(RoundtripError::ParseFailed) if is_error_fixture(path) => {
                must_fail += 1;
            },
            | Err(RoundtripError::ParseFailed) => failures.push(format!(
                "eslint_typescript fixture failed to parse: {display}"
            )),
            | Err(RoundtripError::Roundtrip(message)) => {
                failures.push(message);
            },
        }
        }

        // Rot guard: every skip-list entry must still match at least one case.
        for (entry, matched) in TS_SKIP_LIST
            .iter()
            .zip(TS_SKIP_MATCHED.with_borrow(|matched| matched.to_vec()))
        {
            assert!(
                matched,
                "eslint_typescript skip-list entry no longer matches any fixture: {entry}"
            );
        }

        assert!(
            skipped == TS_SKIP_LIST.len(),
            "eslint_typescript skip-list out of sync (skipped {skipped}, listed {})",
            TS_SKIP_LIST.len(),
        );

        // Backstop: the corpus must stay substantial (upstream layout drift guard).
        assert!(
            parsed + must_fail >= 100,
            "expected a substantial eslint_typescript corpus: {}",
            parsed + must_fail,
        );

        // Backstop: the `_error_` classification must stay exercised.
        assert!(
            must_fail >= 100,
            "expected eslint_typescript _error_ fixtures to be classified: {must_fail}"
        );

        println!(
            "eslint_typescript corpus: {parsed} parsed, {must_fail} must-fail, {skipped} skipped"
        );

        assert!(
            failures.is_empty(),
            "{} eslint_typescript failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
