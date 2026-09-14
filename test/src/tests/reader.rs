use oxc::allocator::Allocator;
use oxc::span::SourceType;
use sonic_rs::Value;

use super::roundtrip::parse_value;

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

    match result {
        | Err(oxc_estree_codec::ReadError::ImportPhaseUnsupported {
            phase,
            ..
        }) => {
            assert_eq!(phase, "bogus");
        },
        | other => panic!("expected ImportPhaseUnsupported, got {other:?}"),
    }
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

#[test]
fn test_read_error_variant_unsupported_node() {
    use oxc_estree_codec::ReadError;

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"BogusNode","start":0,"end":1}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result =
        reader.read(&value, SourceType::from_path("a.js").unwrap(), "");

    match result {
        | Err(ReadError::NodeUnsupported { ty, .. }) => {
            assert_eq!(ty, "BogusNode")
        },
        | other => panic!("expected NodeUnsupported, got {other:?}"),
    }
}

#[test]
fn test_read_error_variant_missing_field() {
    use oxc_estree_codec::ReadError;

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":8,"expression":{"type":"CallExpression","start":0,"end":8,"arguments":[]}}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result =
        reader.read(&value, SourceType::from_path("a.js").unwrap(), "");

    match result {
        | Err(ReadError::FieldMissing { field, ty, .. }) => {
            assert_eq!(field, "callee");
            assert_eq!(ty, "CallExpression");
        },
        | other => panic!("expected FieldMissing, got {other:?}"),
    }
}

#[test]
fn test_read_error_variant_unsupported_operator() {
    use oxc_estree_codec::ReadError;

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":5,"expression":{"type":"BinaryExpression","start":0,"end":5,"operator":"***","left":{"type":"Identifier","name":"a","start":0,"end":1},"right":{"type":"Identifier","name":"b","start":4,"end":5}}}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result =
        reader.read(&value, SourceType::from_path("a.js").unwrap(), "");

    match result {
        | Err(ReadError::OperatorUnsupported { kind, operator, .. }) => {
            assert_eq!(kind, "binary operator");
            assert_eq!(operator, "***");
        },
        | other => panic!("expected OperatorUnsupported, got {other:?}"),
    }
}

#[test]
fn test_read_rejects_identifier_without_name() {
    use oxc_estree_codec::ReadError;

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":1,"expression":{"type":"Identifier","start":0,"end":1}}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result =
        reader.read(&value, SourceType::from_path("a.js").unwrap(), "");

    match result {
        | Err(ReadError::FieldMissing { field, .. }) => {
            assert_eq!(field, "name")
        },
        | other => panic!("expected FieldMissing for `name`, got {other:?}"),
    }
}

#[test]
fn test_read_rejects_identifier_with_non_string_name() {
    use oxc_estree_codec::ReadError;

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":1,"expression":{"type":"Identifier","name":42,"start":0,"end":1}}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result =
        reader.read(&value, SourceType::from_path("a.js").unwrap(), "");

    match result {
        | Err(ReadError::FieldInvalid { field, expected, .. }) => {
            assert_eq!(field, "name");
            assert_eq!(expected, "a string");
        },
        | other => panic!("expected FieldInvalid for `name`, got {other:?}"),
    }
}

#[test]
fn test_read_rejects_hashbang_with_non_string_value() {
    use oxc_estree_codec::ReadError;

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"script","hashbang":{"type":"Hashbang","value":true,"start":0,"end":20},"body":[]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result =
        reader.read(&value, SourceType::from_path("a.js").unwrap(), "");

    match result {
        | Err(ReadError::FieldInvalid { field, .. }) => {
            assert_eq!(field, "value")
        },
        | other => {
            panic!("expected FieldInvalid for hashbang `value`, got {other:?}")
        },
    }
}

#[test]
fn test_read_rejects_string_literal_without_value() {
    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":5,"expression":{"type":"Literal","start":0,"end":5,"raw":"\"abc\""}}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    assert!(
        reader
            .read(&value, SourceType::from_path("a.js").unwrap(), "")
            .is_err(),
        "string literal without `value` must fail loudly"
    );
}

#[test]
fn test_read_rejects_regexp_without_pattern() {
    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":10,"expression":{"type":"Literal","start":0,"end":10,"value":0,"raw":"/ab/g","regex":{"flags":"g"}}}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result =
        reader.read(&value, SourceType::from_path("a.js").unwrap(), "");

    assert!(result.is_err(), "regexp without `pattern` must fail loudly");
}

#[test]
fn test_read_rejects_bigint_without_value() {
    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":6,"expression":{"type":"Literal","start":0,"end":6,"value":null,"raw":"1n"}}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    assert!(
        reader
            .read(&value, SourceType::from_path("a.js").unwrap(), "")
            .is_err(),
        "bigint literal without `bigint` field must fail loudly"
    );
}

#[test]
fn test_read_rejects_meta_property_without_names() {
    use oxc_estree_codec::ReadError;

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":11,"expression":{"type":"MetaProperty","start":0,"end":11,"meta":{"type":"Identifier","name":"import","start":0,"end":6}}}]}"#,
    );

    let reader: oxc_estree_codec::__internal::ProgramReader<'_> =
        oxc_estree_codec::__internal::ProgramReader::new(&allocator);

    let result =
        reader.read(&value, SourceType::from_path("a.js").unwrap(), "");

    match result {
        | Err(ReadError::FieldMissing { field, ty, .. }) => {
            assert_eq!(field, "property");
            assert_eq!(ty, "MetaProperty");
        },
        | other => {
            panic!("expected FieldMissing for `property`, got {other:?}")
        },
    }
}
