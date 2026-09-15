use oxc::ast::ast::{
    TSInterfaceBody, TSInterfaceDeclaration, TSInterfaceHeritage, TSType,
    TSTypeAliasDeclaration, TSTypeName,
};
use oxc::span::Span;

use crate::errors::read::ReadError;
use crate::reader::declaration::function::read_binding_identifier_field;
use crate::reader::engine::context::Cx;
use crate::reader::json::Value;

fn ts_interface_heritage<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSInterfaceHeritage<'a>, ReadError> {
    let expression: TSTypeName<'a> =
        cx.req(node, "expression", |cx, expression_node| {
            crate::reader::ts_types::names::read_ts_type_name_from_expression(
                cx,
                expression_node,
            )
        })?;

    Ok(TSInterfaceHeritage::new(
        cx.span(node),
        expression,
        cx.opt_box(
            node,
            "typeArguments",
            crate::reader::ts_types::types::read_ts_type_parameter_instantiation,
        )?,
        cx.builder(),
    ))
}

pub fn read_ts_type_alias_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypeAliasDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let type_annotation: TSType<'a> = cx.ts(node, "typeAnnotation")?;

    Ok(TSTypeAliasDeclaration::new(
        span,
        read_binding_identifier_field(cx, node, "id")?,
        cx.opt_type_parameters(node)?,
        type_annotation,
        cx.flag(node, "declare"),
        cx.builder(),
    ))
}

pub fn read_ts_interface_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSInterfaceDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let extends: oxc::allocator::Vec<'a, TSInterfaceHeritage<'a>> =
        cx.list(node, "extends", ts_interface_heritage)?;

    let body: TSInterfaceBody<'a> =
        cx.req(node, "body", |cx: &Cx<'a>, body_node: &Value| {
            Ok(TSInterfaceBody::new(
                cx.span(body_node),
                crate::reader::ts_types::signatures::read_ts_signatures(
                    cx, body_node, "body",
                )?,
                cx.builder(),
            ))
        })?;

    Ok(TSInterfaceDeclaration::new(
        span,
        read_binding_identifier_field(cx, node, "id")?,
        cx.opt_type_parameters(node)?,
        extends,
        cx.box_in(body),
        cx.flag(node, "declare"),
        cx.builder(),
    ))
}
