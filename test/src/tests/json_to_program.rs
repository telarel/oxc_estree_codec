use oxc::allocator::Allocator;
use oxc::span::SourceType;

use oxc_estree_compat::{FromJsonOptions, json_to_program};

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
        FromJsonOptions {
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
        FromJsonOptions {
            allocator: &allocator,
            source_type: SourceType::from_path("a.js").unwrap(),
            source_text: "",
        },
    );

    assert!(result.is_err(), "malformed JSON must fail loudly");
}
