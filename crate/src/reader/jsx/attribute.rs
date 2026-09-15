use oxc::ast::ast::{
    Expression, JSXAttribute, JSXAttributeItem, JSXAttributeName,
    JSXAttributeValue, JSXSpreadAttribute,
};
use oxc::span::Span;

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, ty_of};
use crate::reader::json::Value;
use crate::reader::jsx::container::read_jsx_expression_container;
use crate::reader::jsx::element::{
    read_jsx_expression, read_jsx_identifier, read_jsx_namespaced_name,
};
use crate::reader::literal;

fn read_jsx_attribute_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXAttributeName<'a>, ReadError> {
    match ty_of(node) {
        | "JSXIdentifier" => Ok(JSXAttributeName::Identifier(
            cx.box_in(read_jsx_identifier(cx, node)?),
        )),
        | "JSXNamespacedName" => Ok(JSXAttributeName::NamespacedName(
            cx.box_in(read_jsx_namespaced_name(cx, node)?),
        )),
        | _ => Err(cx.err(node)),
    }
}

fn read_jsx_attribute_value<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXAttributeValue<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "Literal" => Ok(JSXAttributeValue::StringLiteral(
            cx.box_in(literal::read_string_literal(cx, node)?),
        )),
        | "JSXExpressionContainer" => {
            Ok(JSXAttributeValue::ExpressionContainer(
                cx.box_in(read_jsx_expression_container(cx, node)?),
            ))
        },
        | "JSXElement" => {
            let element: Expression<'a> = read_jsx_expression(cx, node, ty)?;

            match element {
                | Expression::JSXElement(inner) => {
                    Ok(JSXAttributeValue::Element(inner))
                },
                | _ => Err(cx.err(node)),
            }
        },
        | "JSXFragment" => {
            let fragment: Expression<'a> = read_jsx_expression(cx, node, ty)?;

            match fragment {
                | Expression::JSXFragment(inner) => {
                    Ok(JSXAttributeValue::Fragment(inner))
                },
                | _ => Err(cx.err(node)),
            }
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_jsx_attribute_item<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXAttributeItem<'a>, ReadError> {
    match ty_of(node) {
        | "JSXAttribute" => {
            let span: Span = cx.span(node);

            let name: JSXAttributeName<'a> =
                cx.req(node, "name", read_jsx_attribute_name)?;

            let value: Option<JSXAttributeValue<'a>> =
                cx.opt(node, "value", read_jsx_attribute_value)?;

            let attribute: JSXAttribute<'a> =
                JSXAttribute::new(span, name, value, cx.builder());

            Ok(JSXAttributeItem::Attribute(cx.box_in(attribute)))
        },
        | "JSXSpreadAttribute" => {
            let span: Span = cx.span(node);

            let argument: Expression<'a> = cx.expr(node, "argument")?;

            let spread: JSXSpreadAttribute<'a> =
                JSXSpreadAttribute::new(span, argument, cx.builder());

            Ok(JSXAttributeItem::SpreadAttribute(cx.box_in(spread)))
        },
        | _ => Err(cx.err(node)),
    }
}
