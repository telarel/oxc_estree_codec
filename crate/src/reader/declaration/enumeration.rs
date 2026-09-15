use oxc::ast::ast::{
    IdentifierName, StringLiteral, TSEnumBody, TSEnumDeclaration, TSEnumMember,
    TSEnumMemberName,
};
use oxc::span::Span;

use crate::errors::read::ReadError;
use crate::reader::declaration::function::read_binding_identifier_field;
use crate::reader::engine::context::{Cx, ty_of};
use crate::reader::json::Value;
use crate::reader::literal;

fn ts_enum_member_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
    computed: bool,
) -> Result<TSEnumMemberName<'a>, ReadError> {
    match ty_of(node) {
        | "Identifier" => Ok(TSEnumMemberName::Identifier(cx.box_in(
            IdentifierName::new(cx.span(node), cx.name(node)?, cx.builder()),
        ))),
        | "Literal" => {
            let literal_node: StringLiteral<'a> =
                literal::read_string_literal(cx, node)?;

            // `computed` lives on the `TSEnumMember` node (it is `true` for
            // the `ComputedString`/`ComputedTemplateString` id variants).
            let name: TSEnumMemberName<'a> = if computed {
                TSEnumMemberName::ComputedString(cx.box_in(literal_node))
            } else {
                TSEnumMemberName::String(cx.box_in(literal_node))
            };

            Ok(name)
        },
        | _ => Err(cx.err(node)),
    }
}

fn ts_enum_member<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSEnumMember<'a>, ReadError> {
    let span: Span = cx.span(node);

    let name: TSEnumMemberName<'a> = cx.req(node, "id", |cx, id_node| {
        ts_enum_member_name(cx, id_node, cx.flag(node, "computed"))
    })?;

    Ok(TSEnumMember::new(
        span,
        name,
        cx.opt_expr(node, "initializer")?,
        cx.builder(),
    ))
}

pub fn read_ts_enum_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSEnumDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let body: TSEnumBody<'a> =
        cx.req(node, "body", |cx: &Cx<'a>, body_node: &Value| {
            Ok(TSEnumBody::new(
                cx.span(body_node),
                cx.list(body_node, "members", ts_enum_member)?,
                cx.builder(),
            ))
        })?;

    Ok(TSEnumDeclaration::new(
        span,
        read_binding_identifier_field(cx, node, "id")?,
        body,
        cx.flag(node, "const"),
        cx.flag(node, "declare"),
        cx.builder(),
    ))
}
