use oxc::allocator::Allocator;
use oxc::span::SourceType;
use sonic_rs::Value;

#[test]
fn test_read_rejects_unknown_import_phase() {
    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(
            &allocator,
            "import.source(f)(x);",
            SourceType::from_path("p.ts").unwrap(),
        )
        .parse();

    let json: String = parser_return.program.to_estree_json(true, false);

    let mut value: Value = sonic_rs::from_str::<Value>(&json).unwrap();

    let phase = &mut value["body"][0]["expression"]["callee"]["phase"];

    assert_eq!(phase, "source");

    *phase = Value::from("bogus");

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result = reader.read(&value, parser_return.program.source_type, "");

    assert!(result.is_err(), "unknown phase must fail loudly");
}

#[test]
fn test_read_mutated_tree() {
    let code: &str = "console.log(\"old\");";

    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(
            &allocator,
            code,
            SourceType::from_path("a.ts").unwrap(),
        )
        .parse();

    let json: String = parser_return.program.to_estree_json(true, false);

    let mut value: Value = sonic_rs::from_str::<Value>(&json).unwrap();

    // Simulate a JS transform mutation: rename the callee member.
    let obj = &mut value["body"][0]["expression"]["callee"]["object"];

    assert_eq!(obj["name"], "console");

    obj["name"] = Value::from("consolex");

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let program: oxc::ast::ast::Program<'_> =
        reader.read(&value, parser_return.program.source_type, code).unwrap();

    let out: String = oxc::codegen::Codegen::new().build(&program).code;

    assert!(out.contains("consolex"), "mutation must round-trip: {out}");
}
