use oxc::allocator::Allocator;
use oxc::span::SourceType;

use oxc_estree_codec::{
    JsonToProgramOptions, ProgramToJsonOptions, json_to_program,
    program_to_json,
};

fn parse_to_json(
    code: &str,
    source_type: SourceType,
) -> String {
    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(&allocator, code, source_type).parse();

    assert!(parser_return.diagnostics.is_empty());

    parser_return.program.to_estree_json(true, false)
}

fn read_with_source_type<'a>(
    allocator: &'a Allocator,
    json: &str,
    source_type: SourceType,
) -> oxc::ast::ast::Program<'a> {
    json_to_program(
        json,
        JsonToProgramOptions { allocator, source_type, source_text: "" },
    )
    .unwrap()
}

fn assert_lone_surrogates_roundtrip(
    file: &str,
    code: &str,
) {
    let source_type: SourceType = SourceType::from_path(file).unwrap();

    let json: String = parse_to_json(code, source_type);

    assert!(
        json.contains("\\ud800"),
        "serializer must emit the lone high surrogate escape: {code}: {json}"
    );
    assert!(
        json.contains("\\udc00"),
        "serializer must emit the lone low surrogate escape: {code}: {json}"
    );

    let allocator: Allocator = Allocator::default();

    let program: oxc::ast::ast::Program<'_> =
        read_with_source_type(&allocator, &json, source_type);

    let after: String = program_to_json(&program, ProgramToJsonOptions::new());

    assert_eq!(
        after, json,
        "lone surrogates must round-trip byte-identically: {code}"
    );

    let out: String = oxc::codegen::Codegen::new().build(&program).code;

    let lower_out: String = out.to_ascii_lowercase();

    assert!(
        lower_out.contains("\\ud800"),
        "codegen must keep the high surrogate escape: {code}: {out}"
    );
    assert!(
        lower_out.contains("\\udc00"),
        "codegen must keep the low surrogate escape: {code}: {out}"
    );
}

#[test]
fn test_json_to_program_reads_serializer_output() {
    let code: &str = "const greeting = \"hello\";";

    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(
            &allocator,
            code,
            SourceType::from_path("a.ts").unwrap(),
        )
        .parse();

    assert!(parser_return.diagnostics.is_empty());

    let json: String = parser_return.program.to_estree_json(true, false);

    // The JSON is a local `String`, not arena-allocated: the reader interns
    // strings itself, so the input never needs to outlive the arena.
    let program: oxc::ast::ast::Program<'_> = json_to_program(
        &json,
        JsonToProgramOptions {
            allocator: &allocator,
            source_type: parser_return.program.source_type,
            source_text: code,
        },
    )
    .unwrap();

    let out: String = oxc::codegen::Codegen::new().build(&program).code;

    assert_eq!(out, "const greeting = \"hello\";\n");
}

#[test]
fn test_json_to_program_rejects_invalid_json() {
    let allocator: Allocator = Allocator::default();

    let result = json_to_program(
        "{not json",
        JsonToProgramOptions {
            allocator: &allocator,
            source_type: SourceType::from_path("a.js").unwrap(),
            source_text: "",
        },
    );

    assert!(result.is_err(), "malformed JSON must fail loudly");
}

#[test]
fn test_json_to_program_resolves_unambiguous_module() {
    // `SourceType::from_path` yields `ModuleKind::Unambiguous` for `.tsx`
    // in oxc >= 0.149, which the ESTree serializer cannot emit.
    let ambiguous: SourceType = SourceType::from_path("a.tsx").unwrap();
    assert!(ambiguous.is_unambiguous());

    let json: String = parse_to_json("import x from \"y\";\n", ambiguous);

    let allocator: Allocator = Allocator::default();

    let program: oxc::ast::ast::Program<'_> =
        read_with_source_type(&allocator, &json, ambiguous);

    assert!(program.source_type.is_module());
    assert!(program.source_type.is_typescript());
    assert!(program.source_type.is_jsx());

    let out: String = oxc::codegen::Codegen::new().build(&program).code;

    assert_eq!(out, "import x from \"y\";\n");
}

#[test]
fn test_json_to_program_resolves_unambiguous_script() {
    let ambiguous: SourceType = SourceType::from_path("a.js").unwrap();
    assert!(ambiguous.is_unambiguous());

    let json: String = parse_to_json("const x = 1;\n", ambiguous);

    let allocator: Allocator = Allocator::default();

    let program: oxc::ast::ast::Program<'_> =
        read_with_source_type(&allocator, &json, ambiguous);

    assert!(program.source_type.is_script());
    assert!(program.source_type.is_javascript());

    let out: String = oxc::codegen::Codegen::new().build(&program).code;

    assert_eq!(out, "const x = 1;\n");
}

#[test]
fn test_json_to_program_defaults_unambiguous_to_module() {
    let ambiguous: SourceType = SourceType::from_path("a.ts").unwrap();
    assert!(ambiguous.is_unambiguous());

    let allocator: Allocator = Allocator::default();

    let program: oxc::ast::ast::Program<'_> = read_with_source_type(
        &allocator,
        "{\"type\": \"Program\", \"body\": []}",
        ambiguous,
    );

    assert!(program.source_type.is_module());
    assert!(program.source_type.is_typescript());
}

#[test]
fn test_json_to_program_preserves_lone_surrogates() {
    let cases: &[(&str, &str)] = &[
        ("a.js", "const s = \"a\\uD800b\\uDC00c\";"),
        ("a.js", "const t = `a\\uD800b${x}c\\uDC00d`;"),
        ("a.mjs", "import \"a\\uD800b\\uDC00c\";"),
    ];

    for (file, code) in cases {
        assert_lone_surrogates_roundtrip(file, code);
    }
}
