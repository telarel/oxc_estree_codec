use oxc::allocator::Allocator;
use oxc::span::SourceType;
use sonic_rs::Value;

pub fn parse_value(json: &str) -> Value {
    sonic_rs::Deserializer::from_str(json)
        .use_rawnumber()
        .deserialize::<Value>()
        .unwrap()
}

pub fn parse_to_json(
    code: &str,
    source_type: SourceType,
) -> String {
    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(&allocator, code, source_type).parse();

    assert!(parser_return.diagnostics.is_empty(), "fixture must parse: {code}");

    parser_return.program.to_estree_json(true, false)
}
