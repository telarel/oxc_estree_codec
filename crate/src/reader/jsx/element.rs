use oxc::ast::ast::{
    Expression, IdentifierReference, JSXAttributeItem, JSXChild,
    JSXClosingElement, JSXClosingFragment, JSXElement, JSXElementName,
    JSXFragment, JSXIdentifier, JSXMemberExpression, JSXMemberExpressionObject,
    JSXNamespacedName, JSXOpeningElement, JSXOpeningFragment, JSXSpreadChild,
    JSXText, ThisExpression,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, ty_of};
use crate::reader::json::Value;
use crate::reader::jsx::attribute::read_jsx_attribute_item;
use crate::reader::jsx::container::read_jsx_expression_container;

pub fn read_jsx_identifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXIdentifier<'a>, ReadError> {
    let span: Span = cx.span(node);

    let name_node: &Value =
        node.get("name").ok_or_else(|| cx.missing(node, "name"))?;

    let name: &str = name_node
        .as_str()
        .ok_or_else(|| cx.invalid(node, "name", "a string"))?;

    let name_str: oxc::str::Str<'a> =
        oxc::str::Str::from_str_in(name, cx.builder());

    let identifier: JSXIdentifier<'a> =
        JSXIdentifier::new(span, name_str, cx.builder());

    Ok(identifier)
}

pub fn read_jsx_namespaced_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXNamespacedName<'a>, ReadError> {
    let span: Span = cx.span(node);

    let namespace: JSXIdentifier<'a> =
        cx.req(node, "namespace", read_jsx_identifier)?;

    let name: JSXIdentifier<'a> = cx.req(node, "name", read_jsx_identifier)?;

    let namespaced: JSXNamespacedName<'a> =
        JSXNamespacedName::new(span, namespace, name, cx.builder());

    Ok(namespaced)
}

fn read_jsx_member_expression_object<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXMemberExpressionObject<'a>, ReadError> {
    match ty_of(node) {
        | "JSXIdentifier" | "Identifier" => {
            let name_node: &Value =
                node.get("name").ok_or_else(|| cx.missing(node, "name"))?;

            let name: &str = name_node
                .as_str()
                .ok_or_else(|| cx.invalid(node, "name", "a string"))?;

            if name == "this" {
                let this: ThisExpression =
                    ThisExpression::new(cx.span(node), cx.builder());

                return Ok(JSXMemberExpressionObject::ThisExpression(
                    cx.box_in(this),
                ));
            }

            let name_str: oxc::str::Str<'a> =
                oxc::str::Str::from_str_in(name, cx.builder());

            let reference: IdentifierReference<'a> =
                IdentifierReference::new(cx.span(node), name_str, cx.builder());

            Ok(JSXMemberExpressionObject::IdentifierReference(
                cx.box_in(reference),
            ))
        },
        | "JSXMemberExpression" => {
            Ok(JSXMemberExpressionObject::MemberExpression(
                cx.box_in(read_jsx_member_expression(cx, node)?),
            ))
        },
        | _ => Err(cx.err(node)),
    }
}

fn read_jsx_member_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXMemberExpression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let object: JSXMemberExpressionObject<'a> =
        cx.req(node, "object", read_jsx_member_expression_object)?;

    let property: JSXIdentifier<'a> =
        cx.req(node, "property", read_jsx_identifier)?;

    let member: JSXMemberExpression<'a> =
        JSXMemberExpression::new(span, object, property, cx.builder());

    Ok(member)
}

fn read_jsx_element_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXElementName<'a>, ReadError> {
    match ty_of(node) {
        | "JSXIdentifier" | "Identifier" => Ok(JSXElementName::Identifier(
            cx.box_in(read_jsx_identifier(cx, node)?),
        )),
        | "JSXNamespacedName" => Ok(JSXElementName::NamespacedName(
            cx.box_in(read_jsx_namespaced_name(cx, node)?),
        )),
        | "JSXMemberExpression" => Ok(JSXElementName::MemberExpression(
            cx.box_in(read_jsx_member_expression(cx, node)?),
        )),
        | _ => Err(cx.err(node)),
    }
}

fn read_jsx_opening_element<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXOpeningElement<'a>, ReadError> {
    let span: Span = cx.span(node);

    let name: JSXElementName<'a> =
        cx.req(node, "name", read_jsx_element_name)?;

    let type_arguments: Option<
        oxc::allocator::Box<
            'a,
            oxc::ast::ast::TSTypeParameterInstantiation<'a>,
        >,
    > = cx.opt_type_arguments(node)?;

    let attributes: oxc::allocator::Vec<'a, JSXAttributeItem<'a>> =
        cx.list(node, "attributes", read_jsx_attribute_item)?;

    let opening: JSXOpeningElement<'a> = JSXOpeningElement::new(
        span,
        name,
        type_arguments,
        attributes,
        cx.builder(),
    );

    Ok(opening)
}

fn read_jsx_closing_element<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXClosingElement<'a>, ReadError> {
    let span: Span = cx.span(node);

    let name: JSXElementName<'a> =
        cx.req(node, "name", read_jsx_element_name)?;

    let closing: JSXClosingElement<'a> =
        JSXClosingElement::new(span, name, cx.builder());

    Ok(closing)
}

fn read_jsx_children<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<oxc::allocator::Vec<'a, JSXChild<'a>>, ReadError> {
    cx.list(node, "children", read_jsx_child)
}

fn read_jsx_child<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXChild<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "JSXText" => {
            let span: Span = cx.span(node);

            let value_str: oxc::str::Str<'a> = cx.text(node, "value")?;

            let raw: Option<oxc::str::Str<'a>> = node
                .get("raw")
                .and_then(Value::as_str)
                .map(|r| oxc::str::Str::from_str_in(r, cx.builder()));

            let text: JSXText<'a> =
                JSXText::new(span, value_str, raw, cx.builder());

            Ok(JSXChild::Text(cx.box_in(text)))
        },
        | "JSXElement" => {
            let element: Expression<'a> = read_jsx_expression(cx, node, ty)?;

            match element {
                | Expression::JSXElement(inner) => Ok(JSXChild::Element(inner)),
                | _ => Err(cx.err(node)),
            }
        },
        | "JSXFragment" => {
            let fragment: Expression<'a> = read_jsx_expression(cx, node, ty)?;

            match fragment {
                | Expression::JSXFragment(inner) => {
                    Ok(JSXChild::Fragment(inner))
                },
                | _ => Err(cx.err(node)),
            }
        },
        | "JSXExpressionContainer" => Ok(JSXChild::ExpressionContainer(
            cx.box_in(read_jsx_expression_container(cx, node)?),
        )),
        | "JSXSpreadChild" => {
            let span: Span = cx.span(node);

            let expression_node: Expression<'a> =
                cx.expr(node, "expression")?;

            let spread: JSXSpreadChild<'a> =
                JSXSpreadChild::new(span, expression_node, cx.builder());

            Ok(JSXChild::Spread(cx.box_in(spread)))
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_jsx_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
    ty: &str,
) -> Result<Expression<'a>, ReadError> {
    match ty {
        | "JSXElement" => {
            let span: Span = cx.span(node);

            let opening: JSXOpeningElement<'a> =
                cx.req(node, "openingElement", read_jsx_opening_element)?;

            let children: oxc::allocator::Vec<'a, JSXChild<'a>> =
                read_jsx_children(cx, node)?;

            let closing: Option<
                oxc::allocator::Box<'a, JSXClosingElement<'a>>,
            > = cx
                .opt(node, "closingElement", read_jsx_closing_element)?
                .map(|element| cx.box_in(element));

            let element: JSXElement<'a> = JSXElement::new(
                span,
                cx.box_in(opening),
                children,
                closing,
                cx.builder(),
            );

            Ok(Expression::JSXElement(cx.box_in(element)))
        },
        | "JSXFragment" => {
            let span: Span = cx.span(node);

            let opening_node: &Value =
                node.get("openingFragment").ok_or_else(|| cx.err(node))?;

            let opening: JSXOpeningFragment =
                JSXOpeningFragment::new(cx.span(opening_node), cx.builder());

            let children: oxc::allocator::Vec<'a, JSXChild<'a>> =
                read_jsx_children(cx, node)?;

            let closing_node: &Value =
                node.get("closingFragment").ok_or_else(|| cx.err(node))?;

            let closing: JSXClosingFragment =
                JSXClosingFragment::new(cx.span(closing_node), cx.builder());

            let fragment: JSXFragment<'a> = JSXFragment::new(
                span,
                opening,
                children,
                closing,
                cx.builder(),
            );

            Ok(Expression::JSXFragment(cx.box_in(fragment)))
        },
        | _ => Err(cx.err(node)),
    }
}
