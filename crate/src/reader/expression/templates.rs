use oxc::ast::ast::{TemplateElement, TemplateElementValue, TemplateLiteral};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::engine::context::Cx;
use crate::reader::json::Value;

pub fn read_template_element<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TemplateElement<'a>, ReadError> {
    let mut span: Span = cx.span(node);

    let tail: bool = cx.flag(node, "tail");

    span.start += 1;

    span.end -= if tail { 1 } else { 2 };

    let value_node: &Value = node.get("value").ok_or_else(|| cx.err(node))?;

    let raw_node: &Value =
        value_node.get("raw").ok_or_else(|| cx.missing(value_node, "raw"))?;

    let raw: &str = raw_node
        .as_str()
        .ok_or_else(|| cx.invalid(value_node, "raw", "a string"))?;

    let raw_str: oxc::str::Str<'a> =
        oxc::str::Str::from_str_in(raw, cx.builder());

    let cooked: Option<oxc::str::Str<'a>> = match value_node.get("cooked") {
        | Some(cooked_node) if cooked_node.is_str() => {
            let cooked_str: &str = cooked_node.as_str().unwrap_or_default();
            Some(oxc::str::Str::from_str_in(cooked_str, cx.builder()))
        },
        | _ => None,
    };

    let lone_surrogates: bool = cooked
        .as_ref()
        .is_some_and(|cooked| cooked.as_str().contains('\u{FFFD}'));

    let value: TemplateElementValue<'a> =
        TemplateElementValue { raw: raw_str, cooked };

    let element: TemplateElement<'a> =
        TemplateElement::new_with_lone_surrogates(
            span,
            value,
            tail,
            lone_surrogates,
            cx.builder(),
        );

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
