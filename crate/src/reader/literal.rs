use oxc::allocator::GetAllocator;
use oxc::ast::ast::{
    BigIntLiteral, BooleanLiteral, Expression, NullLiteral, NumericLiteral,
    RegExp, RegExpFlags, RegExpLiteral, RegExpPattern, StringLiteral,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::engine::context::Cx;
use crate::reader::json::Value;

/// Check whether a string is in oxc's in-memory lone-surrogate encoding:
/// every `U+FFFD` must be immediately followed by 4 lowercase hex chars.
fn is_lone_surrogate_encoded(s: &str) -> bool {
    if !s.contains('\u{FFFD}') {
        return false;
    }

    let bytes: &[u8] = s.as_bytes();

    let mut i: usize = 0;

    while i + 3 <= bytes.len() {
        if bytes[i] == 0xEF && bytes[i + 1] == 0xBF && bytes[i + 2] == 0xBD {
            if !matches!(
                bytes.get(i + 3..i + 7),
                |Some([
                    b'0'..=b'9' | b'a'..=b'f',
                    b'0'..=b'9' | b'a'..=b'f',
                    b'0'..=b'9' | b'a'..=b'f',
                    b'0'..=b'9' | b'a'..=b'f',
                ])
            ) {
                return false;
            }

            i += 7;

            continue;
        }

        i += 1;
    }

    true
}

fn number_base(raw: &str) -> oxc::syntax::number::NumberBase {
    if raw.starts_with("0x") || raw.starts_with("0X") {
        oxc::syntax::number::NumberBase::Hex
    } else if raw.starts_with("0o") || raw.starts_with("0O") {
        oxc::syntax::number::NumberBase::Octal
    } else if raw.starts_with("0b") || raw.starts_with("0B") {
        oxc::syntax::number::NumberBase::Binary
    } else if raw.contains('.') || raw.contains('e') || raw.contains('E') {
        oxc::syntax::number::NumberBase::Float
    } else {
        oxc::syntax::number::NumberBase::Decimal
    }
}

fn bigint_base(raw: &str) -> oxc::syntax::number::BigintBase {
    if raw.starts_with("0x") || raw.starts_with("0X") {
        oxc::syntax::number::BigintBase::Hex
    } else if raw.starts_with("0o") || raw.starts_with("0O") {
        oxc::syntax::number::BigintBase::Octal
    } else if raw.starts_with("0b") || raw.starts_with("0B") {
        oxc::syntax::number::BigintBase::Binary
    } else {
        oxc::syntax::number::BigintBase::Decimal
    }
}

pub fn regexp_flags(flags: &str) -> RegExpFlags {
    let mut bits: RegExpFlags = RegExpFlags::empty();

    for flag in flags.chars() {
        bits |= match flag {
            | 'g' => RegExpFlags::G,
            | 'i' => RegExpFlags::I,
            | 'm' => RegExpFlags::M,
            | 's' => RegExpFlags::S,
            | 'u' => RegExpFlags::U,
            | 'y' => RegExpFlags::Y,
            | 'd' => RegExpFlags::D,
            | 'v' => RegExpFlags::V,
            | _ => RegExpFlags::empty(),
        };
    }

    bits
}

pub fn read_literal<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let raw: Option<&str> = node.get("raw").and_then(Value::as_str);

    if let Some(regex) = node.get("regex").filter(|r| r.is_object()) {
        let pattern_node: &Value =
            regex.get("pattern").ok_or_else(|| cx.missing(regex, "pattern"))?;

        let pattern: &str = pattern_node
            .as_str()
            .ok_or_else(|| cx.invalid(regex, "pattern", "a string"))?;

        let flags_node: &Value =
            regex.get("flags").ok_or_else(|| cx.missing(regex, "flags"))?;

        let flags: &str = flags_node
            .as_str()
            .ok_or_else(|| cx.invalid(regex, "flags", "a string"))?;

        let pattern_text: &'a str = cx.builder.allocator().alloc_str(pattern);

        let flags_bits: RegExpFlags = regexp_flags(flags);

        let regex: RegExp<'a> = RegExp {
            pattern: RegExpPattern { pattern: None, text: pattern_text.into() },
            flags: flags_bits,
        };

        let raw_str: Option<oxc::str::Str<'a>> =
            raw.map(|r| oxc::str::Str::from_str_in(r, cx.builder()));

        let lit: RegExpLiteral<'a> =
            RegExpLiteral::new(span, regex, raw_str, cx.builder());

        return Ok(Expression::RegExpLiteral(cx.box_in(lit)));
    }

    if let Some(bigint) = node.get("bigint").filter(|b| b.is_str()) {
        let raw_str: Option<oxc::str::Str<'a>> =
            raw.map(|r| oxc::str::Str::from_str_in(r, cx.builder()));

        let value_str: oxc::str::Str<'a> = oxc::str::Str::from_str_in(
            bigint.as_str().unwrap_or_default(),
            cx.builder(),
        );

        let base: oxc::syntax::number::BigintBase =
            bigint_base(raw.unwrap_or_default());

        let lit: BigIntLiteral<'a> =
            BigIntLiteral::new(span, value_str, raw_str, base, cx.builder());

        return Ok(Expression::BigIntLiteral(cx.box_in(lit)));
    }

    match node.get("value") {
        | Some(value_node) if value_node.is_str() => {
            let s: &str = value_node.as_str().unwrap_or_default();

            let raw_str: Option<oxc::str::Str<'a>> =
                raw.map(|r| oxc::str::Str::from_str_in(r, cx.builder()));

            let lit: StringLiteral<'a> = if is_lone_surrogate_encoded(s) {
                StringLiteral::new_with_lone_surrogates(
                    span,
                    oxc::str::Str::from_str_in(s, cx.builder()),
                    raw_str,
                    true,
                    cx.builder(),
                )
            } else {
                StringLiteral::new(
                    span,
                    cx.builder.allocator().alloc_str(s),
                    raw_str,
                    cx.builder(),
                )
            };

            Ok(Expression::StringLiteral(cx.box_in(lit)))
        },
        | Some(value_node) if value_node.is_number() => {
            let value: f64 = value_node
                .as_raw_number()
                .and_then(|number| number.as_str().parse::<f64>().ok())
                .unwrap_or(f64::INFINITY);

            let base: oxc::syntax::number::NumberBase =
                number_base(raw.unwrap_or(""));

            let raw_str: Option<oxc::str::Str<'a>> =
                raw.map(|r| oxc::str::Str::from_str_in(r, cx.builder()));

            let lit: NumericLiteral<'a> =
                NumericLiteral::new(span, value, raw_str, base, cx.builder());

            Ok(Expression::NumericLiteral(cx.box_in(lit)))
        },
        | Some(value_node) if value_node.is_boolean() => {
            let b: bool = value_node
                .as_bool()
                .ok_or_else(|| cx.invalid(node, "value", "a boolean"))?;

            let lit: BooleanLiteral =
                BooleanLiteral::new(span, b, cx.builder());

            Ok(Expression::BooleanLiteral(cx.box_in(lit)))
        },
        | Some(value_node) if value_node.is_null() && raw == Some("null") => {
            // Spec-optional: `raw` is the only discriminator for null literals.
            let lit: NullLiteral = NullLiteral::new(span, cx.builder());

            Ok(Expression::NullLiteral(cx.box_in(lit)))
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_string_literal<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<StringLiteral<'a>, ReadError> {
    let span: Span = cx.span(node);

    let value: Option<&str> = node.get("value").and_then(Value::as_str);

    let raw: Option<&str> = node.get("raw").and_then(Value::as_str);

    match value {
        | Some(v) => {
            let raw_str: Option<oxc::str::Str<'a>> =
                raw.map(|r| oxc::str::Str::from_str_in(r, cx.builder()));

            let lit: StringLiteral<'a> = if is_lone_surrogate_encoded(v) {
                StringLiteral::new_with_lone_surrogates(
                    span,
                    oxc::str::Str::from_str_in(v, cx.builder()),
                    raw_str,
                    true,
                    cx.builder(),
                )
            } else {
                StringLiteral::new(
                    span,
                    cx.builder.allocator().alloc_str(v),
                    raw_str,
                    cx.builder(),
                )
            };

            Ok(lit)
        },
        | None => Err(cx.err(node)),
    }
}
