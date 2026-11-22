use std::fmt::Debug;

use oxc::allocator::Allocator;
use oxc::ast::ast::Program;
use oxc::span::SourceType;
use oxc_estree_codec::{
    JsonToProgramOptions, ReadError, SpanEncoding, json_to_program,
};
use sonic_rs::Value;

const JS_FILE: &str = "a.js";

pub(crate) fn read_program<'a>(
    allocator: &'a Allocator,
    value: &Value,
    source: &'a str,
    source_type: SourceType,
    span_encoding: SpanEncoding,
) -> Result<Program<'a>, ReadError> {
    let json: String =
        sonic_rs::to_string(value).expect("ESTree value must serialize");

    json_to_program(
        &json,
        JsonToProgramOptions {
            allocator,
            source_type,
            source_text: source,
            span_encoding,
        },
    )
}

pub(crate) fn read_program_js<'a>(
    allocator: &'a Allocator,
    value: &Value,
    source: &'a str,
    span_encoding: SpanEncoding,
) -> Result<Program<'a>, ReadError> {
    read_program(
        allocator,
        value,
        source,
        SourceType::from_path(JS_FILE).unwrap(),
        span_encoding,
    )
}

pub(crate) fn assert_field_invalid<T: Debug>(
    result: Result<T, ReadError>,
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
