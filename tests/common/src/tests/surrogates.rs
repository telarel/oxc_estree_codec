use oxc::allocator::Allocator;
use oxc::ast::ast::{
    Expression, JSXChild, Program, Statement, StringLiteral, TemplateElement,
};
use oxc::span::SourceType;
use oxc_estree_codec::{
    JsonToProgramOptions, ProgramToJsonOptions, SpanEncoding, json_to_program,
    program_to_json,
};
use sonic_rs::{JsonValueTrait, Value};

use test_helpers::json::{parse_to_json, parse_value};

const JS_FILE: &str = "a.js";
const JSX_FILE: &str = "a.jsx";

fn read_code_and_json<'a>(
    allocator: &'a Allocator,
    code: &'a str,
    file: &str,
) -> (String, Program<'a>) {
    let source_type: SourceType = SourceType::from_path(file).unwrap();

    let json: String = parse_to_json(code, source_type);

    let program: Program<'a> = json_to_program(
        &json,
        JsonToProgramOptions {
            allocator,
            source_type,
            source_text: code,
            span_encoding: SpanEncoding::Utf8Bytes,
        },
    )
    .unwrap_or_else(|error| panic!("source must read: {code}: {error:?}"));

    (json, program)
}

fn read_code<'a>(
    allocator: &'a Allocator,
    code: &'a str,
    file: &str,
) -> Program<'a> {
    read_code_and_json(allocator, code, file).1
}

fn const_initializer<'a, 'p>(program: &'p Program<'a>) -> &'p Expression<'a> {
    let Statement::VariableDeclaration(declaration) = &program.body[0] else {
        panic!("expected a variable declaration");
    };

    let declarator = &declaration.declarations[0];

    declarator.init.as_ref().expect("the declaration must have an initializer")
}

fn string_literal<'a, 'p>(program: &'p Program<'a>) -> &'p StringLiteral<'a> {
    let Expression::StringLiteral(literal) = const_initializer(program) else {
        panic!("expected a string literal");
    };

    literal
}

fn template_element<'a, 'p>(
    program: &'p Program<'a>
) -> &'p TemplateElement<'a> {
    let Expression::TemplateLiteral(literal) = const_initializer(program)
    else {
        panic!("expected a template literal");
    };

    assert_eq!(literal.quasis.len(), 1, "expected exactly one quasi");

    &literal.quasis[0]
}

fn assert_roundtrips(code: &str) {
    let allocator: Allocator = Allocator::default();

    let (json, program): (String, Program<'_>) =
        read_code_and_json(&allocator, code, JS_FILE);

    let after: String = program_to_json(&program, ProgramToJsonOptions::new());

    assert_eq!(
        after, json,
        "the Program must round-trip byte-identically: {code}"
    );
}

/// Serialize `program`. Assert the fields of the emitted `StringLiteral`.
fn assert_emitted_string_literal(
    program: &Program<'_>,
    value: &str,
    raw: &str,
) {
    let after: String = program_to_json(program, ProgramToJsonOptions::new());

    let emitted: Value = parse_value(&after);

    let literal: &Value = &emitted["body"][0]["declarations"][0]["init"];

    assert_eq!(
        literal["value"].as_str(),
        Some(value),
        "the round-trip must re-emit the decoded value: {after}"
    );

    assert_eq!(
        literal["raw"].as_str(),
        Some(raw),
        "the round-trip must re-emit the raw source text: {after}"
    );
}

#[test]
fn test_literal_replacement_char_followed_by_hex() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const s = \"\u{FFFD}beef\";";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let literal: &StringLiteral<'_> = string_literal(&program);

    assert_eq!(
        literal.value.as_str(),
        Some("\u{FFFD}beef"),
        "literal U+FFFD followed by hex must stay text: {code}"
    );

    assert_eq!(
        literal.raw.as_ref().map(|raw| raw.as_str()),
        Some("\"\u{FFFD}beef\""),
        "the raw value must stay byte-identical to the source: {code}"
    );
}

#[test]
fn test_literal_replacement_char_followed_by_surrogate_hex() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const s = \"\u{FFFD}d800\";";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let literal: &StringLiteral<'_> = string_literal(&program);

    assert_eq!(
        literal.value.as_str(),
        Some("\u{FFFD}d800"),
        "literal U+FFFD followed by `d800` must not become a lone surrogate: {code}"
    );

    assert_eq!(
        literal.raw.as_ref().map(|raw| raw.as_str()),
        Some("\"\u{FFFD}d800\""),
        "the raw value must stay byte-identical to the source: {code}"
    );

    assert!(
        !literal.raw.as_ref().is_some_and(|raw| raw.as_str().contains("fffd")),
        "the raw value must not expose the `fffd` marker: {code}"
    );
}

#[test]
fn test_literal_replacement_char_followed_by_fffd_text() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const s = \"\u{FFFD}fffd\";";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let literal: &StringLiteral<'_> = string_literal(&program);

    assert_eq!(
        literal.value.as_str(),
        Some("\u{FFFD}fffd"),
        "the self-referential marker text must stay literal text: {code}"
    );

    assert_eq!(
        literal.raw.as_ref().map(|raw| raw.as_str()),
        Some("\"\u{FFFD}fffd\""),
        "the raw value must stay byte-identical to the source: {code}"
    );

    assert_emitted_string_literal(&program, "\u{FFFD}fffd", "\"\u{FFFD}fffd\"");
}

#[test]
fn test_literal_replacement_char_at_end_of_document() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const s = \"\u{FFFD}\";";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let literal: &StringLiteral<'_> = string_literal(&program);

    assert_eq!(
        literal.value.as_str(),
        Some("\u{FFFD}"),
        "a trailing literal U+FFFD must stay text: {code}"
    );

    assert_eq!(
        literal.raw.as_ref().map(|raw| raw.as_str()),
        Some("\"\u{FFFD}\""),
        "the raw value must stay byte-identical to the source: {code}"
    );

    assert!(
        !literal.raw.as_ref().is_some_and(|raw| raw.as_str().contains("fffd")),
        "the raw value must not expose the `fffd` marker: {code}"
    );

    assert_roundtrips(code);
}

#[test]
fn test_literal_replacement_char_followed_by_ffff() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const s = \"\u{FFFD}ffff\";";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let literal: &StringLiteral<'_> = string_literal(&program);

    assert_eq!(
        literal.value.as_str(),
        Some("\u{FFFD}ffff"),
        "literal U+FFFD followed by `ffff` must stay text: {code}"
    );
}

#[test]
fn test_lone_surrogates_still_roundtrip() {
    assert_roundtrips("const s = \"a\\uD800b\\uDC00c\";");
    assert_roundtrips("const t = `a\\uD800b${x}c\\uDC00d`;");
}

#[test]
fn test_mixed_literal_and_lone_surrogate() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const s = \"\u{FFFD}beef\\uD800x\u{FFFD}ffff\";";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let literal: &StringLiteral<'_> = string_literal(&program);

    assert!(
        literal.value.as_str().is_none(),
        "the decoded value must keep the lone surrogate: {code}"
    );

    assert_eq!(
        literal.raw.as_ref().map(|raw| raw.as_str()),
        Some("\"\u{FFFD}beef\\uD800x\u{FFFD}ffff\""),
        "the raw value must stay byte-identical to the source: {code}"
    );

    assert_roundtrips(code);
}

#[test]
fn test_template_raw_with_literal_replacement_char() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const t = `\u{FFFD}d800`;";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let element: &TemplateElement<'_> = template_element(&program);

    assert_eq!(
        element.value.raw.as_str(),
        "\u{FFFD}d800",
        "the template raw value must not expose the `fffd` marker: {code}"
    );

    assert_eq!(
        element.value.cooked.as_ref().and_then(|cooked| cooked.as_str()),
        Some("\u{FFFD}d800"),
        "the template cooked value must decode the literal text: {code}"
    );

    let start: usize = element.span.start as usize;
    let end: usize = element.span.end as usize;

    assert_eq!(
        &code[start..end],
        "\u{FFFD}d800",
        "the template element span must cover the source text: {code}"
    );
}

#[test]
fn test_directive_with_literal_replacement_char() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "\"\u{FFFD}beef\";\n";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    assert_eq!(program.directives.len(), 1, "expected one directive");

    let directive = &program.directives[0];

    assert_eq!(
        directive.directive.as_str(),
        "\u{FFFD}beef",
        "the directive atom must not expose the `fffd` marker: {code}"
    );

    assert_eq!(
        directive.expression.value.as_str(),
        Some("\u{FFFD}beef"),
        "the directive expression must decode the literal text: {code}"
    );
}

#[test]
fn test_hashbang_with_literal_replacement_char() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "#!env \u{FFFD}beef\nconst x = 1;\n";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let hashbang = program.hashbang.expect("expected a hashbang");

    assert_eq!(
        hashbang.value.as_str(),
        "env \u{FFFD}beef",
        "the hashbang value must not expose the `fffd` marker: {code}"
    );
}

#[test]
fn test_jsx_text_with_literal_replacement_char() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const el = <div>\u{FFFD}d800</div>;";

    let program: Program<'_> = read_code(&allocator, code, JSX_FILE);

    let Expression::JSXElement(element) = const_initializer(&program) else {
        panic!("expected a JSX element");
    };

    let JSXChild::Text(text) = &element.children[0] else {
        panic!("expected JSX text");
    };

    assert_eq!(
        text.value.as_str(),
        "\u{FFFD}d800",
        "the JSX text value must not expose the `fffd` marker: {code}"
    );

    assert_eq!(
        text.raw.as_ref().map(|raw| raw.as_str()),
        Some("\u{FFFD}d800"),
        "the JSX text raw value must not expose the `fffd` marker: {code}"
    );
}

#[test]
fn test_raw_source_text_is_not_corrupted() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const s = \"\u{FFFD}ffff\";";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let literal: &StringLiteral<'_> = string_literal(&program);

    assert_eq!(
        literal.raw.as_ref().map(|raw| raw.as_str()),
        Some("\"\u{FFFD}ffff\""),
        "the raw value must stay byte-identical to the source: {code}"
    );

    assert!(
        !literal.raw.as_ref().is_some_and(|raw| raw.as_str().contains("fffd")),
        "the raw value must not expose the `fffd` marker: {code}"
    );
}

#[test]
fn test_escaped_replacement_char_followed_by_hex() {
    // `parse_to_json` emits literal U+FFFD bytes, so build the JSON by hand to
    // exercise the pre-pass branch for `\uFFFD` escapes.
    let json: &str = concat!(
        "{\"type\":\"Program\",\"sourceType\":\"script\",\"body\":[",
        "{\"type\":\"ExpressionStatement\",\"expression\":",
        "{\"type\":\"Literal\",\"value\":\"\\ufffdbeef\"}}]}",
    );

    let allocator: Allocator = Allocator::default();

    let program: Program<'_> = json_to_program(
        json,
        JsonToProgramOptions {
            allocator: &allocator,
            source_type: SourceType::from_path(JS_FILE).unwrap(),
            source_text: "",
            span_encoding: SpanEncoding::Utf8Bytes,
        },
    )
    .expect("the JSON escape must read");

    let Statement::ExpressionStatement(statement) = &program.body[0] else {
        panic!("expected an expression statement");
    };

    let Expression::StringLiteral(literal) = &statement.expression else {
        panic!("expected a string literal");
    };

    assert_eq!(
        literal.value.as_str(),
        Some("\u{FFFD}beef"),
        "a JSON `\\ufffd` escape followed by hex must stay text"
    );
}

#[test]
fn test_regex_pattern_with_literal_replacement_char() {
    let allocator: Allocator = Allocator::default();

    let code: &str = "const r = /\u{FFFD}d800/u;";

    let program: Program<'_> = read_code(&allocator, code, JS_FILE);

    let Expression::RegExpLiteral(literal) = const_initializer(&program) else {
        panic!("expected a regexp literal");
    };

    assert_eq!(
        literal.regex.pattern.text.as_str(),
        "\u{FFFD}d800",
        "the regexp pattern must not expose the `fffd` marker: {code}"
    );

    assert_eq!(
        literal.raw.as_ref().map(|raw| raw.as_str()),
        Some("/\u{FFFD}d800/u"),
        "the regexp raw value must stay byte-identical to the source: {code}"
    );
}
