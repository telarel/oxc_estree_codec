use oxc::allocator::{Box as ArenaBox, Vec as ArenaVec};
use oxc::ast::ast::{
    ArrayPattern, AssignmentPattern, BindingIdentifier, BindingPattern,
    BindingProperty, BindingRestElement, ObjectPattern,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, nodes, ty_of};
use crate::reader::json::Value;

fn binding_property<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingProperty<'a>, ReadError> {
    Ok(BindingProperty::new(
        cx.span(node),
        cx.property_key(node, "key")?,
        cx.pattern(node, "value")?,
        cx.flag(node, "shorthand"),
        cx.flag(node, "computed"),
        cx.builder(),
    ))
}

fn array_pattern<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingPattern<'a>, ReadError> {
    let span: Span = cx.span(node);

    let mut elements: ArenaVec<'a, Option<BindingPattern<'a>>> =
        ArenaVec::new_in(cx.builder());

    let mut rest: Option<ArenaBox<'a, BindingRestElement<'a>>> = None;

    cx.list(node, "elements", |cx, item| {
        if ty_of(item) == "RestElement" {
            let argument: BindingPattern<'a> =
                cx.req(item, "argument", read_binding_pattern)?;

            let rest_element: BindingRestElement<'a> =
                BindingRestElement::new(cx.span(item), argument, cx.builder());

            rest = Some(cx.box_in(rest_element));

            return Ok(());
        }

        if item.is_null() {
            elements.push(None);
            return Ok(());
        }

        elements.push(Some(read_binding_pattern(cx, item)?));

        Ok(())
    })?;

    let pattern: ArrayPattern<'a> =
        ArrayPattern::new(span, elements, rest, cx.builder());

    Ok(BindingPattern::ArrayPattern(cx.box_in(pattern)))
}

fn object_pattern<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingPattern<'a>, ReadError> {
    let span: Span = cx.span(node);

    let mut properties: ArenaVec<'a, BindingProperty<'a>> =
        ArenaVec::new_in(cx.builder());

    let mut rest: Option<ArenaBox<'a, BindingRestElement<'a>>> = None;

    cx.list(node, "properties", |cx, item| {
        if ty_of(item) == "RestElement" {
            let argument: BindingPattern<'a> =
                cx.req(item, "argument", read_binding_pattern)?;

            let rest_element: BindingRestElement<'a> =
                BindingRestElement::new(cx.span(item), argument, cx.builder());

            rest = Some(cx.box_in(rest_element));

            return Ok(());
        }

        properties.push(binding_property(cx, item)?);

        Ok(())
    })?;

    let pattern: ObjectPattern<'a> =
        ObjectPattern::new(span, properties, rest, cx.builder());

    Ok(BindingPattern::ObjectPattern(cx.box_in(pattern)))
}

pub fn read_binding_pattern<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingPattern<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "ArrayPattern" => array_pattern(cx, node),
        | "ObjectPattern" => object_pattern(cx, node),
        | _ => nodes! { cx, node, ty, BindingPattern :
            "Identifier" => BindingIdentifier: BindingIdentifier::new [
                cx.span(node),
                cx.name(node)?,
            ];
            "AssignmentPattern" => AssignmentPattern: AssignmentPattern::new [
                cx.span(node),
                cx.pattern(node, "left")?,
                cx.expr(node, "right")?,
            ];
        },
    }
}
