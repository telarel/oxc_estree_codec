use oxc::allocator::Allocator;
use oxc::ast::ast::{Expression, Program, Statement};
use oxc::span::{SourceType, Span};
use oxc_estree_codec::{
    ProgramToJsonOptions, ReadError, SpanEncoding, program_to_json,
};
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};

use test_helpers::json::parse_value;

use crate::tests::support::{assert_field_invalid, read_program};

const FILE: &str = "a.js";
const TS_FILE: &str = "a.ts";

fn template_element(
    raw: &str,
    tail: bool,
    start: u64,
    end: u64,
) -> Value {
    let mut element: Value = Value::new_object();

    element["type"] = Value::from("TemplateElement");
    element["value"]["raw"] = Value::from(raw);
    element["value"]["cooked"] = Value::from(raw);
    element["tail"] = Value::from(tail);
    element["start"] = Value::from(start);
    element["end"] = Value::from(end);

    element
}

fn template_program(quasis: Vec<Value>) -> Value {
    let mut literal: Value = Value::new_object();

    literal["type"] = Value::from("TemplateLiteral");
    literal["quasis"] = Value::from(quasis);
    literal["expressions"] = Value::new_array();

    let mut statement: Value = Value::new_object();

    statement["type"] = Value::from("ExpressionStatement");
    statement["expression"] = literal;

    let mut program: Value = Value::new_object();

    program["type"] = Value::from("Program");
    program["sourceType"] = Value::from("module");
    program["body"] = Value::from(vec![statement]);

    program
}

fn read_single_span(
    allocator: &Allocator,
    element: Value,
    source: &str,
) -> Result<Span, ReadError> {
    let value: Value = template_program(vec![element]);

    let program: Program<'_> = read_program(
        allocator,
        &value,
        source,
        SourceType::from_path(FILE).unwrap(),
        SpanEncoding::Utf8Bytes,
    )?;

    let Statement::ExpressionStatement(statement) = &program.body[0] else {
        panic!("expected an expression statement");
    };

    let Expression::TemplateLiteral(literal) = &statement.expression else {
        panic!("expected a template literal");
    };

    assert_eq!(literal.quasis.len(), 1, "expected exactly one quasi");

    Ok(literal.quasis[0].span)
}

fn assert_span(
    allocator: &Allocator,
    element: Value,
    source: &str,
    expected: Span,
) {
    let span: Span = read_single_span(allocator, element, source)
        .expect("template element must read");

    assert_eq!(span, expected);
}

fn collect_template_elements(
    value: &Value,
    elements: &mut Vec<(String, u64, u64)>,
) {
    if let Some(object) = value.as_object() {
        if object.get(&"type").and_then(Value::as_str)
            == Some("TemplateElement")
        {
            let raw: &str = object
                .get(&"value")
                .and_then(|value| value.get("raw"))
                .and_then(Value::as_str)
                .expect("template element must have a raw value");

            let start: u64 = object
                .get(&"start")
                .and_then(Value::as_u64)
                .expect("template element must have a start");

            let end: u64 = object
                .get(&"end")
                .and_then(Value::as_u64)
                .expect("template element must have an end");

            elements.push((raw.to_string(), start, end));
        }

        for (_, child) in object.iter() {
            collect_template_elements(child, elements);
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            collect_template_elements(child, elements);
        }
    }
}

#[test]
fn test_content_only_non_tail_span() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "`foo`";

    let element: Value = template_element("foo", false, 1, 4);

    assert_span(&allocator, element, source, Span::new(1, 4));
}

#[test]
fn test_content_only_tail_span() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "`foo`";

    let element: Value = template_element("foo", true, 1, 4);

    assert_span(&allocator, element, source, Span::new(1, 4));
}

#[test]
fn test_delimiter_inclusive_non_tail_span() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "`foo${x}`";

    let element: Value = template_element("foo", false, 0, 6);

    assert_span(&allocator, element, source, Span::new(1, 4));
}

#[test]
fn test_delimiter_inclusive_tail_span() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "`foo`";

    let element: Value = template_element("foo", true, 0, 5);

    assert_span(&allocator, element, source, Span::new(1, 4));
}

#[test]
fn test_empty_quasi_does_not_underflow() {
    let allocator: Allocator = Allocator::default();

    assert_span(
        &allocator,
        template_element("", false, 1, 1),
        "``",
        Span::new(1, 1),
    );

    assert_span(
        &allocator,
        template_element("", false, 0, 3),
        "`${x}`",
        Span::new(1, 1),
    );

    assert_span(
        &allocator,
        template_element("", true, 0, 2),
        "``",
        Span::new(1, 1),
    );
}

#[test]
fn test_unicode_raw_span() {
    let allocator: Allocator = Allocator::default();

    let cases: [(&str, Value); 3] = [
        ("`é${x}`", template_element("é", false, 1, 3)),
        ("`é${x}`", template_element("é", false, 0, 5)),
        ("`é`", template_element("é", true, 0, 4)),
    ];

    for (source, element) in cases {
        let span: Span = read_single_span(&allocator, element, source)
            .expect("unicode raw value must read");

        assert_eq!(&source[span.start as usize..span.end as usize], "é");
    }
}

#[test]
fn test_mismatched_span_is_rejected() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "`foo`";

    let result: Result<Span, ReadError> = read_single_span(
        &allocator,
        template_element("foo", false, 1, 5),
        source,
    );

    assert_field_invalid(result, "start", "a span matching value.raw");
}

#[test]
fn test_crlf_tail_span_normalizes() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "`a\r\nb`";
    let raw: &str = "a\nb";

    let element: Value = template_element(raw, true, 0, 6);

    let span: Span = read_single_span(&allocator, element, source)
        .expect("CRLF delimiter-inclusive span must read");

    assert_eq!(span, Span::new(1, 5));
    assert_eq!(&source[1..5], "a\r\nb");
    assert_eq!(&source[1..5].replace("\r\n", "\n"), raw);
}

#[test]
fn test_crlf_content_only_span() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "`a\r\nb`";
    let raw: &str = "a\nb";

    let element: Value = template_element(raw, true, 1, 5);

    let span: Span = read_single_span(&allocator, element, source)
        .expect("CRLF content-only span must read");

    assert_eq!(span, Span::new(1, 5));
    assert_eq!(&source[span.start as usize..span.end as usize], "a\r\nb");
}

#[test]
fn test_crlf_escape_raw() {
    let allocator: Allocator = Allocator::default();
    let source: &str = "`a\\r\r\nb`";
    let raw: &str = "a\\r\nb";

    let cases: [(u64, u64); 2] = [(1, 7), (0, 8)];

    for (start, end) in cases {
        let element: Value = template_element(raw, true, start, end);

        let span: Span = read_single_span(&allocator, element, source)
            .expect("raw value with an escaped \\r must read");

        assert_eq!(span, Span::new(1, 7));
        assert_eq!(&source[1..7], "a\\r\r\nb");
        assert_eq!(&source[1..7].replace("\r\n", "\n"), raw);
    }
}

#[test]
fn test_source_free_length_fallback() {
    let allocator: Allocator = Allocator::default();

    let cases: [(Value, Span); 4] = [
        (template_element("foo", false, 1, 4), Span::new(1, 4)),
        (template_element("foo", true, 1, 4), Span::new(1, 4)),
        (template_element("foo", false, 0, 6), Span::new(1, 4)),
        (template_element("foo", true, 0, 5), Span::new(1, 4)),
    ];

    for (element, expected) in cases {
        let span: Span = read_single_span(&allocator, element, "")
            .expect("span must read when `source_text` is empty");

        assert_eq!(span, expected);
    }
}

#[test]
fn test_source_free_crlf_span_is_rejected() {
    let allocator: Allocator = Allocator::default();

    // The length fallback cannot account for CRLF without source text.
    let result: Result<Span, ReadError> =
        read_single_span(&allocator, template_element("a\nb", true, 1, 5), "");

    assert_field_invalid(result, "start", "a span matching value.raw");
}

#[test]
fn test_include_ts_fields_false_roundtrip() {
    let source: &str = "const a = `head ${x} mid ${y} tail`;\nconst b = `é${x}`;\nconst c = `a\\n${x}\\tb`;\ntype T = `a${string}b`;\nconst d = ``;\nconst e = `crlf\r\nline`;\nconst f = `cr\rline`;\n";

    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(
            &allocator,
            source,
            SourceType::from_path(TS_FILE).unwrap(),
        )
        .parse();

    assert!(
        parser_return.diagnostics.is_empty(),
        "fixture must parse: {source}"
    );

    let json: String = program_to_json(
        &parser_return.program,
        ProgramToJsonOptions { include_ts_fields: false, ranges: false },
    );

    assert!(
        json.contains("\"TSTemplateLiteralType\""),
        "the input must contain a `TSTemplateLiteralType` node: {json}"
    );

    let value: Value = parse_value(&json);

    let program: Program<'_> = read_program(
        &allocator,
        &value,
        source,
        parser_return.program.source_type,
        SpanEncoding::Utf8Bytes,
    )
    .expect("include_ts_fields = false output must read");

    let after: String = program_to_json(
        &program,
        ProgramToJsonOptions { include_ts_fields: false, ranges: false },
    );

    let decoded: Value = parse_value(&after);

    let mut elements: Vec<(String, u64, u64)> = Vec::new();

    collect_template_elements(&decoded, &mut elements);

    assert_eq!(elements.len(), 12, "expected every quasi to read");

    assert!(
        elements.iter().any(|(raw, ..)| raw == "é"),
        "the fixture must include a non-ASCII raw value"
    );
    assert!(
        elements.iter().any(|(raw, ..)| raw == "a\\n"),
        "the fixture must include an escaped raw value"
    );
    assert!(
        elements.iter().any(|(raw, ..)| raw.is_empty()),
        "the fixture must include an empty raw value"
    );
    assert!(
        elements.iter().any(|(raw, ..)| raw == "crlf\nline"),
        "the fixture must include a normalized CRLF raw value"
    );
    assert!(
        elements.iter().any(|(raw, ..)| raw == "cr\nline"),
        "the fixture must include a normalized CR raw value"
    );

    for (raw, start, end) in elements {
        let slice: &str = &source[start as usize..end as usize];

        assert!(
            slice == raw
                || slice.replace("\r\n", "\n").replace('\r', "\n") == raw,
            "template span must cover the raw value after the codec converts CRLF and CR to LF: {slice:?} compared with {raw:?}"
        );
    }
}
