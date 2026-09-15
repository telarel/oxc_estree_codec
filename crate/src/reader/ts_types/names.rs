use oxc::allocator::Box as ArenaBox;
use oxc::ast::ast::{
    BindingIdentifier, Expression, IdentifierName, IdentifierReference,
    ObjectExpression, StringLiteral, TSImportType, TSImportTypeQualifiedName,
    TSImportTypeQualifier, TSQualifiedName, TSType, TSTypeName,
    TSTypeQueryExprName, ThisExpression,
};
use oxc::span::Span;

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, ty_of};
use crate::reader::expression::expressions;
use crate::reader::json::Value;
use crate::reader::ts_types::types::read_ts_type;

pub fn read_identifier_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<IdentifierName<'a>, ReadError> {
    Ok(IdentifierName::new(cx.span(node), cx.name(node)?, cx.builder()))
}

pub fn read_binding_identifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingIdentifier<'a>, ReadError> {
    Ok(BindingIdentifier::new(cx.span(node), cx.name(node)?, cx.builder()))
}

pub fn read_ts_type_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypeName<'a>, ReadError> {
    match ty_of(node) {
        | "Identifier" => {
            let reference: IdentifierReference<'a> = IdentifierReference::new(
                cx.span(node),
                cx.name(node)?,
                cx.builder(),
            );

            Ok(TSTypeName::IdentifierReference(cx.box_in(reference)))
        },
        | "ThisExpression" => {
            let this: ThisExpression =
                ThisExpression::new(cx.span(node), cx.builder());

            Ok(TSTypeName::ThisExpression(cx.box_in(this)))
        },
        | "TSQualifiedName" => {
            let qualified: TSQualifiedName<'a> = TSQualifiedName::new(
                cx.span(node),
                cx.req(node, "left", read_ts_type_name)?,
                cx.req(node, "right", read_identifier_name)?,
                cx.builder(),
            );

            Ok(TSTypeName::QualifiedName(cx.box_in(qualified)))
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_ts_type_name_from_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypeName<'a>, ReadError> {
    match ty_of(node) {
        | "Identifier" => read_ts_type_name(cx, node),
        | "MemberExpression" => {
            if cx.flag(node, "computed") || cx.flag(node, "optional") {
                return Err(cx.err(node));
            }
            let object: TSTypeName<'a> =
                cx.req(node, "object", read_ts_type_name_from_expression)?;

            let property: IdentifierName<'a> =
                cx.req(node, "property", read_identifier_name)?;

            let qualified: TSQualifiedName<'a> = TSQualifiedName::new(
                cx.span(node),
                object,
                property,
                cx.builder(),
            );

            Ok(TSTypeName::QualifiedName(cx.box_in(qualified)))
        },
        | _ => Err(cx.err(node)),
    }
}

fn read_ts_import_type_qualifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSImportTypeQualifier<'a>, ReadError> {
    match ty_of(node) {
        | "Identifier" => Ok(TSImportTypeQualifier::Identifier(
            cx.box_in(read_identifier_name(cx, node)?),
        )),
        | "TSQualifiedName" => {
            let left: TSImportTypeQualifier<'a> =
                cx.req(node, "left", read_ts_import_type_qualifier)?;

            let right: IdentifierName<'a> =
                cx.req(node, "right", read_identifier_name)?;

            let qualified: TSImportTypeQualifiedName<'a> =
                TSImportTypeQualifiedName::new(
                    cx.span(node),
                    left,
                    right,
                    cx.builder(),
                );

            Ok(TSImportTypeQualifier::QualifiedName(cx.box_in(qualified)))
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_ts_import_type<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let span: Span = cx.span(node);

    let source: StringLiteral<'a> = cx.string_literal(node, "source")?;

    let options: Option<ArenaBox<'a, ObjectExpression<'a>>> =
        cx.opt(node, "options", |cx: &Cx<'a>, options_node: &Value| {
            match expressions::read_expression(cx, options_node)? {
                | Expression::ObjectExpression(object) => Ok(object),
                | _ => Err(cx.err(options_node)),
            }
        })?;

    let qualifier: Option<TSImportTypeQualifier<'a>> =
        cx.opt(node, "qualifier", read_ts_import_type_qualifier)?;

    let import_type: TSImportType<'a> = TSImportType::new(
        span,
        source,
        options,
        qualifier,
        cx.opt_type_arguments(node)?,
        cx.builder(),
    );

    Ok(TSType::TSImportType(cx.box_in(import_type)))
}

pub fn read_ts_type_query_expr_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypeQueryExprName<'a>, ReadError> {
    match ty_of(node) {
        | "TSImportType" => match read_ts_type(cx, node)? {
            | TSType::TSImportType(boxed) => {
                Ok(TSTypeQueryExprName::TSImportType(boxed))
            },
            | _ => Err(cx.err(node)),
        },
        | "ThisExpression" => {
            let this: ThisExpression =
                ThisExpression::new(cx.span(node), cx.builder());

            Ok(TSTypeQueryExprName::ThisExpression(cx.box_in(this)))
        },
        | "Identifier" | "TSQualifiedName" => {
            let name: TSTypeName<'a> = read_ts_type_name(cx, node)?;
            Ok(TSTypeQueryExprName::from(name))
        },
        | _ => Err(cx.err(node)),
    }
}
