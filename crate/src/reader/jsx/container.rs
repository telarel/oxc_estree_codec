use oxc::ast::ast::{
    Expression, JSXEmptyExpression, JSXExpression, JSXExpressionContainer,
};
use oxc::span::Span;

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, ty_of};
use crate::reader::expression::expressions;
use crate::reader::json::Value;

fn read_jsx_expression_inner<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXExpression<'a>, ReadError> {
    if ty_of(node) == "JSXEmptyExpression" {
        let empty: JSXEmptyExpression =
            JSXEmptyExpression::new(cx.span(node), cx.builder());

        return Ok(JSXExpression::EmptyExpression(cx.box_in(empty)));
    }

    let expression_node: Expression<'a> =
        expressions::read_expression(cx, node)?;

    Ok(JSXExpression::from(expression_node))
}

pub fn read_jsx_expression_container<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<JSXExpressionContainer<'a>, ReadError> {
    let span: Span = cx.span(node);

    let expression_node: JSXExpression<'a> =
        cx.req(node, "expression", |cx: &Cx<'a>, inner: &Value| {
            read_jsx_expression_inner(cx, inner)
        })?;

    let container: JSXExpressionContainer<'a> =
        JSXExpressionContainer::new(span, expression_node, cx.builder());

    Ok(container)
}
