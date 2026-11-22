use std::borrow::Cow;
use std::iter::Peekable;
use std::str::Chars;

use oxc::ast::ast::{TemplateElement, TemplateElementValue, TemplateLiteral};
use oxc::span::Span;
use sonic_rs::{JsonValueTrait, Value};

use crate::errors::read::ReadError;
use crate::reader::engine::context::Cx;
use crate::reader::json;

/// ESTree normalizes CRLF and CR to LF in `value.raw`. Normalize a source
/// slice the same way before comparing it with the raw value.
fn normalize_line_terminators(text: &str) -> Cow<'_, str> {
    if !text.contains('\r') {
        return Cow::Borrowed(text);
    }

    let mut normalized: String = String::with_capacity(text.len());

    let mut chars: Peekable<Chars<'_>> = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            | '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }

                normalized.push('\n');
            },
            | _ => normalized.push(ch),
        }
    }

    Cow::Owned(normalized)
}

fn is_span_raw_match(
    source_text: &str,
    span: Span,
    raw: &str,
) -> bool {
    let Some(content): Option<&str> =
        source_text.get(span.start as usize..span.end as usize)
    else {
        return false;
    };

    content == raw || normalize_line_terminators(content).as_ref() == raw
}

fn shifted_span(
    declared: Span,
    tail: bool,
) -> Option<Span> {
    let start: u32 = declared.start.checked_add(1)?;

    let end: u32 = if tail {
        declared.end.checked_sub(1)?
    } else {
        declared.end.checked_sub(2)?
    };

    (start <= end).then_some(Span::new(start, end))
}

fn element_span(
    cx: &Cx<'_>,
    node: &Value,
    raw: &str,
    raw_len: u32,
    tail: bool,
) -> Result<Span, ReadError> {
    let Some(declared): Option<Span> = cx.checked_span_opt(node)? else {
        return Ok(Span::new(0, 0));
    };

    let source_text: &str = cx.source_text();

    if !source_text.is_empty() {
        if is_span_raw_match(source_text, declared, raw) {
            return Ok(declared);
        }

        if let Some(shifted) = shifted_span(declared, tail)
            && is_span_raw_match(source_text, shifted, raw)
        {
            return Ok(shifted);
        }

        return Err(cx.invalid(node, "start", "a span matching value.raw"));
    }

    let declared_len: u32 = declared.size();

    if declared_len == raw_len {
        return Ok(declared);
    }

    // Delimiter-inclusive spans add the opening backtick and either the
    // closing backtick (tail) or `${` (non-tail).
    let delimiter_len: u32 = if tail { 2 } else { 3 };

    if raw_len.checked_add(delimiter_len) != Some(declared_len) {
        return Err(cx.invalid(node, "start", "a span matching value.raw"));
    }

    shifted_span(declared, tail)
        .ok_or_else(|| cx.invalid(node, "start", "a span matching value.raw"))
}

pub fn read_template_element<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TemplateElement<'a>, ReadError> {
    let tail: bool = cx.flag(node, "tail");

    let value_node: &Value = node.get("value").ok_or_else(|| cx.err(node))?;

    let raw_node: &Value =
        value_node.get("raw").ok_or_else(|| cx.missing(value_node, "raw"))?;

    let raw: &str = raw_node
        .as_str()
        .ok_or_else(|| cx.invalid(value_node, "raw", "a string"))?;

    let raw_len: u32 = u32::try_from(raw.len())
        .map_err(|_| cx.invalid(value_node, "raw", "at most u32 bytes"))?;

    let span: Span = element_span(cx, node, raw, raw_len, tail)?;

    let raw_str: oxc::str::Str<'a> =
        oxc::str::Str::from_str_in(raw, cx.builder());

    let cooked: Option<oxc::str::JSStr<'a>> = match value_node.get("cooked") {
        | Some(cooked_node) if cooked_node.is_str() => {
            let cooked_str: &str = cooked_node.as_str().unwrap_or_default();
            Some(json::read_js_str(cooked_str, cx.builder()))
        },
        | _ => None,
    };

    let value: TemplateElementValue<'a> =
        TemplateElementValue { raw: raw_str, cooked };

    let element: TemplateElement<'a> =
        TemplateElement::new(span, value, tail, cx.builder());

    Ok(element)
}

pub fn read_template_literal<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TemplateLiteral<'a>, ReadError> {
    Ok(TemplateLiteral::new(
        cx.span(node),
        cx.list(node, "quasis", read_template_element)?,
        cx.exprs(node, "expressions")?,
        cx.builder(),
    ))
}
