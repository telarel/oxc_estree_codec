use oxc::allocator::Allocator;
use oxc::ast::ast::{BindingPattern, Expression, Program, Statement};
use oxc::span::{SourceType, Span};
use oxc_estree_codec::{
    JsonToProgramOptions, ReadError, SpanEncoding, json_to_program,
};
use sonic_rs::{JsonValueMutTrait, JsonValueTrait, Object, Value};

use test_helpers::json::{parse_to_json, parse_value};

const FILE: &str = "a.js";

fn parse_estree(source: &str) -> Value {
    let source_type: SourceType = SourceType::from_path(FILE).unwrap();

    let json: String = parse_to_json(source, source_type);

    parse_value(&json)
}

fn utf16_offset(
    source: &str,
    byte: u64,
) -> u64 {
    source[..byte as usize].encode_utf16().count() as u64
}

fn read_program<'a>(
    allocator: &'a Allocator,
    value: &Value,
    source: &'a str,
    span_encoding: SpanEncoding,
) -> Result<Program<'a>, ReadError> {
    let json: String =
        sonic_rs::to_string(value).expect("ESTree value must serialize");

    json_to_program(
        &json,
        JsonToProgramOptions {
            allocator,
            source_type: SourceType::from_path(FILE).unwrap(),
            source_text: source,
            span_encoding,
        },
    )
}

fn walk_objects(
    value: &mut Value,
    visit: &mut impl FnMut(&mut Object),
) {
    if let Some(object) = value.as_object_mut() {
        visit(object);

        for (_, child) in object.iter_mut() {
            walk_objects(child, visit);
        }
    } else if let Some(array) = value.as_array_mut() {
        for child in array.as_mut_slice() {
            walk_objects(child, visit);
        }
    }
}

fn map_offsets_to_utf16(
    value: &mut Value,
    source: &str,
) {
    walk_objects(value, &mut |object: &mut Object| {
        let start: Option<u64> = object.get(&"start").and_then(Value::as_u64);
        let end: Option<u64> = object.get(&"end").and_then(Value::as_u64);

        if let (Some(start), Some(end)) = (start, end) {
            object.insert("start", utf16_offset(source, start));
            object.insert("end", utf16_offset(source, end));
        }
    });
}

fn replace_offsets_with_range(
    value: &mut Value,
    source: &str,
) {
    walk_objects(value, &mut |object: &mut Object| {
        let start: Option<u64> = object.get(&"start").and_then(Value::as_u64);
        let end: Option<u64> = object.get(&"end").and_then(Value::as_u64);

        if let (Some(start), Some(end)) = (start, end) {
            let range: Value = Value::from(vec![
                utf16_offset(source, start),
                utf16_offset(source, end),
            ]);
            object.remove(&"start");
            object.remove(&"end");
            object.insert("range", range);
        }
    });
}

fn replace_offsets_with_byte_range(value: &mut Value) {
    walk_objects(value, &mut |object: &mut Object| {
        let start: Option<u64> = object.get(&"start").and_then(Value::as_u64);
        let end: Option<u64> = object.get(&"end").and_then(Value::as_u64);

        if let (Some(start), Some(end)) = (start, end) {
            let range: Value = Value::from(vec![start, end]);
            object.remove(&"start");
            object.remove(&"end");
            object.insert("range", range);
        }
    });
}

fn insert_bogus_range(value: &mut Value) {
    walk_objects(value, &mut |object: &mut Object| {
        if object.get(&"start").is_some() && object.get(&"end").is_some() {
            object.insert("range", Value::from(vec![0_u64, 1_u64]));
        }
    });
}

fn identifier_value() -> Value {
    parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","expression":{"type":"Identifier","name":"x"}}]}"#,
    )
}

fn identifier_mut(value: &mut Value) -> &mut Value {
    &mut value["body"][0]["expression"]
}

fn assert_field_invalid(
    result: Result<Program<'_>, ReadError>,
    field: &'static str,
    expected: &'static str,
) {
    match result {
        | Err(ReadError::FieldInvalid {
            field: actual_field,
            expected: actual_expected,
            ..
        }) => {
            assert_eq!(actual_field, field);
            assert_eq!(actual_expected, expected);
        },
        | other => panic!("expected FieldInvalid for `{field}`, got {other:?}"),
    }
}

fn target_span(program: &Program<'_>) -> Span {
    let statement: &Statement<'_> = &program.body[0];

    let Statement::VariableDeclaration(declaration) = statement else {
        panic!("expected a variable declaration");
    };

    let declarator = &declaration.declarations[0];

    let BindingPattern::BindingIdentifier(identifier) = &declarator.id else {
        panic!("expected a binding identifier");
    };

    identifier.span
}

#[test]
fn test_span_encoding_new_and_default_are_utf8_bytes() {
    assert_eq!(SpanEncoding::new(), SpanEncoding::Utf8Bytes);
    assert_eq!(SpanEncoding::default(), SpanEncoding::Utf8Bytes);
}

#[test]
fn test_utf16_spans_map_to_utf8_bytes() {
    let source: &str = "// 😀\nconst target = 1;\n";

    let allocator: Allocator = Allocator::default();

    let mut value: Value = parse_estree(source);

    map_offsets_to_utf16(&mut value, source);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect("UTF-16 offsets must read");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
}

#[test]
fn test_range_only_spans_map_to_utf8_bytes() {
    let source: &str = "// 😀\nconst target = 1;\n";

    let allocator: Allocator = Allocator::default();

    let mut value: Value = parse_estree(source);

    replace_offsets_with_range(&mut value, source);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect("UTF-16 ranges must read");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
}

#[test]
fn test_utf8_range_only_spans_are_byte_offsets() {
    let source: &str = "// 😀\nconst target = 1;\n";

    let allocator: Allocator = Allocator::default();

    let mut value: Value = parse_estree(source);

    replace_offsets_with_byte_range(&mut value);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf8Bytes)
            .expect("UTF-8 ranges must read");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
}

#[test]
fn test_start_end_take_precedence_over_range() {
    let source: &str = "// 😀\nconst target = 1;\n";

    let allocator: Allocator = Allocator::default();

    let mut value: Value = parse_estree(source);

    map_offsets_to_utf16(&mut value, source);
    insert_bogus_range(&mut value);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect("start/end must take precedence over range");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
}

#[test]
fn test_utf16_maps_bmp_and_combining_mark_offsets() {
    let source: &str = "// e\u{0301}\u{00e9}\nconst target = 1;\n";

    let allocator: Allocator = Allocator::default();

    let mut value: Value = parse_estree(source);

    map_offsets_to_utf16(&mut value, source);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect("BMP and combining mark offsets must read");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
}

#[test]
fn test_utf16_maps_offsets_before_non_ascii_suffix() {
    let source: &str = "const target = 1;\n// 😀\n";

    let allocator: Allocator = Allocator::default();

    let mut value: Value = parse_estree(source);

    map_offsets_to_utf16(&mut value, source);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect("offsets before a non-ASCII suffix must read");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
}

#[test]
fn test_utf16_rejects_span_inside_surrogate_pair() {
    let source: &str = "😀";

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","start":0,"end":2,"expression":{"type":"Identifier","name":"x","start":1,"end":2}}]}"#,
    );

    let error: ReadError =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect_err("an offset inside a surrogate pair must fail");

    match error {
        | ReadError::FieldInvalid { field, expected, .. } => {
            assert_eq!(field, "start");
            assert_eq!(
                expected,
                "a UTF-16 offset on a UTF-8 char boundary within source_text"
            );
        },
        | other => panic!("expected FieldInvalid, got {other:?}"),
    }
}

#[test]
fn test_utf16_rejects_span_inside_surrogate_pair_for_end() {
    let source: &str = "😀";

    let allocator: Allocator = Allocator::default();

    let mut value: Value = identifier_value();
    identifier_mut(&mut value)["start"] = Value::from(0_u64);
    identifier_mut(&mut value)["end"] = Value::from(1_u64);

    let result: Result<Program<'_>, ReadError> =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units);

    assert_field_invalid(
        result,
        "end",
        "a UTF-16 offset on a UTF-8 char boundary within source_text",
    );
}

#[test]
fn test_utf16_rejects_reversed_span() {
    let source: &str = "target";

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","expression":{"type":"Identifier","name":"x","start":6,"end":0}}]}"#,
    );

    let error: ReadError =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect_err("a reversed span must fail");

    match error {
        | ReadError::FieldInvalid { field, expected, .. } => {
            assert_eq!(field, "end");
            assert_eq!(expected, "an offset at or after start");
        },
        | other => panic!("expected FieldInvalid, got {other:?}"),
    }
}

#[test]
fn test_utf16_rejects_offset_past_source() {
    let allocator: Allocator = Allocator::default();

    for source in ["target", "😀"] {
        let mut value: Value = identifier_value();
        identifier_mut(&mut value)["start"] = Value::from(0_u64);
        identifier_mut(&mut value)["end"] = Value::from(100_u64);

        let result: Result<Program<'_>, ReadError> =
            read_program(&allocator, &value, source, SpanEncoding::Utf16Units);

        assert_field_invalid(
            result,
            "end",
            "a UTF-16 offset on a UTF-8 char boundary within source_text",
        );
    }
}

#[test]
fn test_utf8_spans_are_identity() {
    let source: &str = "// 😀\nconst target = 1;\n";

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_estree(source);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf8Bytes)
            .expect("UTF-8 offsets must read");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
    assert_eq!(
        span.start,
        u32::try_from(source.find("target").unwrap()).unwrap()
    );
}

#[test]
fn test_missing_spans_degrade_to_zero() {
    let allocator: Allocator = Allocator::default();

    let value: Value = parse_value(
        r#"{"type":"Program","sourceType":"module","body":[{"type":"ExpressionStatement","expression":{"type":"Identifier","name":"x"}}]}"#,
    );

    let encodings: [SpanEncoding; 2] =
        [SpanEncoding::Utf8Bytes, SpanEncoding::Utf16Units];

    for span_encoding in encodings {
        let program: Program<'_> =
            read_program(&allocator, &value, "", span_encoding)
                .expect("missing spans must not fail the read");

        assert_eq!(program.span, Span::new(0, 0));

        let Statement::ExpressionStatement(statement) = &program.body[0] else {
            panic!("expected an expression statement");
        };

        let Expression::Identifier(identifier) = &statement.expression else {
            panic!("expected an identifier");
        };

        assert_eq!(identifier.span, Span::new(0, 0));
    }
}

#[test]
fn test_utf16_rejects_non_zero_span_with_empty_source() {
    let allocator: Allocator = Allocator::default();

    let mut value: Value = identifier_value();
    identifier_mut(&mut value)["start"] = Value::from(1_u64);
    identifier_mut(&mut value)["end"] = Value::from(2_u64);

    let result: Result<Program<'_>, ReadError> =
        read_program(&allocator, &value, "", SpanEncoding::Utf16Units);

    assert_field_invalid(
        result,
        "start",
        "a UTF-16 offset on a UTF-8 char boundary within source_text",
    );

    let mut value: Value = identifier_value();
    identifier_mut(&mut value)["start"] = Value::from(0_u64);
    identifier_mut(&mut value)["end"] = Value::from(1_u64);

    let result: Result<Program<'_>, ReadError> =
        read_program(&allocator, &value, "", SpanEncoding::Utf16Units);

    assert_field_invalid(
        result,
        "end",
        "a UTF-16 offset on a UTF-8 char boundary within source_text",
    );
}

#[test]
fn test_utf8_rejects_span_above_u32_max() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "target";
    let above: u64 = u64::from(u32::MAX) + 1;

    let mut value: Value = identifier_value();
    identifier_mut(&mut value)["start"] = Value::from(above);
    identifier_mut(&mut value)["end"] = Value::from(above + 1);

    let result: Result<Program<'_>, ReadError> =
        read_program(&allocator, &value, source, SpanEncoding::Utf8Bytes);

    assert_field_invalid(result, "start", "an offset that fits in u32");

    let mut value: Value = identifier_value();
    identifier_mut(&mut value)["start"] = Value::from(0_u64);
    identifier_mut(&mut value)["end"] = Value::from(above);

    let result: Result<Program<'_>, ReadError> =
        read_program(&allocator, &value, source, SpanEncoding::Utf8Bytes);

    assert_field_invalid(result, "end", "an offset that fits in u32");
}

#[test]
fn test_span_rejects_partial_start_and_end() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "target";

    let mut value: Value = identifier_value();
    identifier_mut(&mut value)["start"] = Value::from(0_u64);

    let result: Result<Program<'_>, ReadError> =
        read_program(&allocator, &value, source, SpanEncoding::new());

    assert_field_invalid(result, "end", "a number");

    let mut value: Value = identifier_value();
    identifier_mut(&mut value)["end"] = Value::from(1_u64);

    let result: Result<Program<'_>, ReadError> =
        read_program(&allocator, &value, source, SpanEncoding::new());

    assert_field_invalid(result, "start", "a number");

    let mut value: Value = identifier_value();
    identifier_mut(&mut value)["start"] = Value::from("0");
    identifier_mut(&mut value)["end"] = Value::from(1_u64);

    let result: Result<Program<'_>, ReadError> =
        read_program(&allocator, &value, source, SpanEncoding::new());

    assert_field_invalid(result, "start", "a number");
}

#[test]
fn test_span_rejects_malformed_range() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "target";

    let ranges: [Value; 3] = [
        Value::from("not-an-array"),
        Value::from(vec![0_u64]),
        Value::from(vec![Value::from(0_u64), Value::from("1")]),
    ];

    for range in ranges {
        let mut value: Value = identifier_value();
        identifier_mut(&mut value)["range"] = range;

        let result: Result<Program<'_>, ReadError> =
            read_program(&allocator, &value, source, SpanEncoding::new());

        assert_field_invalid(result, "range", "[start, end]");
    }
}

#[test]
fn test_utf16_ascii_source_is_identity() {
    let source: &str = "const target = 1;\n";

    let allocator: Allocator = Allocator::default();

    let value: Value = parse_estree(source);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect("ASCII UTF-16 offsets must read");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
}

#[test]
fn test_utf16_maps_three_byte_bmp_offsets() {
    let source: &str = "// 日本語\nconst target = 1;\n";

    let allocator: Allocator = Allocator::default();

    let mut value: Value = parse_estree(source);

    map_offsets_to_utf16(&mut value, source);

    let program: Program<'_> =
        read_program(&allocator, &value, source, SpanEncoding::Utf16Units)
            .expect("three-byte BMP offsets must read");

    let span: Span = target_span(&program);

    assert_eq!(&source[span.start as usize..span.end as usize], "target");
}
