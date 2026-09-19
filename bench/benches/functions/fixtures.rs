use oxc::allocator::Allocator;
use oxc::span::SourceType;
use sonic_rs::Value;

use oxc_estree_codec::JsonToProgramOptions;

pub struct Fixture {
    pub name: &'static str,
    pub json: String,
    pub value: Value,
    pub source_type: SourceType,
    pub source: &'static str,
}

pub fn prepare(
    name: &'static str,
    source: &'static str,
) -> Fixture {
    let source_type: SourceType =
        SourceType::from_path(format!("{name}.tsx")).unwrap();

    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(&allocator, source, source_type)
            .with_options(oxc::parser::ParseOptions {
                parse_regular_expression: true,
                ..oxc::parser::ParseOptions::default()
            })
            .parse();

    assert!(
        parser_return.diagnostics.is_empty(),
        "bench fixture {name} must parse: {:?}",
        parser_return.diagnostics
    );

    let json: String = parser_return.program.to_estree_json(true, false);

    let value: Value = sonic_rs::from_slice(json.as_bytes()).unwrap();

    let code: String =
        oxc::codegen::Codegen::new().build(&parser_return.program).code;

    assert!(
        !code.is_empty(),
        "bench fixture {name} must codegen non-empty output"
    );

    let program: oxc::ast::ast::Program<'_> =
        oxc_estree_codec::json_to_program(
            &json,
            JsonToProgramOptions {
                allocator: &allocator,
                source_type,
                source_text: source,
            },
        )
        .expect("bench fixture round-trips");

    let roundtrip_code: String =
        oxc::codegen::Codegen::new().build(&program).code;

    assert!(
        !roundtrip_code.is_empty(),
        "bench fixture {name} must round-trip to non-empty codegen output"
    );

    Fixture { name, json, value, source_type, source }
}

pub fn fixtures() -> [Fixture; 2] {
    let react_page: Fixture =
        prepare("react_page", include_str!("../../fixtures/react_page.tsx"));

    let react_page_large: Fixture = prepare(
        "react_page_large",
        include_str!("../../fixtures/react_page_large.tsx"),
    );

    [react_page, react_page_large]
}
