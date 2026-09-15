use oxc::allocator::Box as ArenaBox;
use oxc::ast::ast::{
    BindingIdentifier, Expression, TSArrayType, TSConditionalType,
    TSIndexedAccessType, TSInferType, TSIntersectionType, TSLiteral,
    TSLiteralType, TSMappedType, TSMappedTypeModifierOperator,
    TSNamedTupleMember, TSOptionalType, TSRestType, TSTemplateLiteralType,
    TSTupleElement, TSTupleType, TSType, TSTypeAnnotation, TSTypeLiteral,
    TSTypeOperator, TSTypeOperatorOperator, TSTypeParameter,
    TSTypeParameterDeclaration, TSTypeParameterInstantiation, TSTypePredicate,
    TSTypeQuery, TSTypeReference, TSUnionType,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, nodes, ty_of};
use crate::reader::expression::expressions;
use crate::reader::expression::templates;
use crate::reader::json::Value;
use crate::reader::ts_types::names::{
    read_binding_identifier, read_identifier_name, read_ts_import_type,
    read_ts_type_name, read_ts_type_query_expr_name,
};
use crate::reader::ts_types::signatures::{
    read_ts_constructor_type, read_ts_function_type,
    read_ts_parenthesized_type, read_ts_signatures,
    read_ts_type_predicate_name,
};

fn mapped_type_modifier(
    cx: &Cx<'_>,
    value: Option<&Value>,
) -> Result<Option<TSMappedTypeModifierOperator>, ReadError> {
    let modifier: Option<TSMappedTypeModifierOperator> = match value {
        | None => None,
        | Some(v) if v.is_null() => None,
        // `false` is the serialized form of `None` (see oxc's
        // `TSMappedTypeOptional` serializer), so it reads back as absent.
        | Some(v) if v.is_boolean() && v.as_bool() == Some(false) => None,
        | Some(v) if v.is_boolean() && v.as_bool() == Some(true) => {
            Some(TSMappedTypeModifierOperator::True)
        },
        | Some(v) if v.is_str() && v.as_str() == Some("+") => {
            Some(TSMappedTypeModifierOperator::Plus)
        },
        | Some(v) if v.is_str() && v.as_str() == Some("-") => {
            Some(TSMappedTypeModifierOperator::Minus)
        },
        | Some(other) => {
            return Err(ReadError::ValueUnsupported {
                kind: "mapped type modifier",
                value: other.to_string(),
                path: cx.path_string(),
            });
        },
    };

    Ok(modifier)
}

fn read_ts_literal<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSLiteral<'a>, ReadError> {
    let literal_node: TSLiteral<'a> =
        match expressions::read_expression(cx, node)? {
            | Expression::BooleanLiteral(inner) => {
                TSLiteral::BooleanLiteral(inner)
            },
            | Expression::NumericLiteral(inner) => {
                TSLiteral::NumericLiteral(inner)
            },
            | Expression::BigIntLiteral(inner) => {
                TSLiteral::BigIntLiteral(inner)
            },
            | Expression::StringLiteral(inner) => {
                TSLiteral::StringLiteral(inner)
            },
            | Expression::TemplateLiteral(inner) => {
                TSLiteral::TemplateLiteral(inner)
            },
            | Expression::UnaryExpression(inner) => {
                TSLiteral::UnaryExpression(inner)
            },
            | _ => return Err(cx.err(node)),
        };

    Ok(literal_node)
}

fn read_ts_type_operator<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let operator: TSTypeOperatorOperator =
        match node.get("operator").and_then(Value::as_str) {
            | Some("keyof") => TSTypeOperatorOperator::Keyof,
            | Some("unique") => TSTypeOperatorOperator::Unique,
            | Some("readonly") => TSTypeOperatorOperator::Readonly,
            | other => {
                return Err(ReadError::OperatorUnsupported {
                    kind: "type operator",
                    operator: other.unwrap_or("<missing>").to_string(),
                    path: cx.path_string(),
                });
            },
        };

    let operator_type: TSTypeOperator<'a> = TSTypeOperator::new(
        cx.span(node),
        operator,
        cx.ts(node, "typeAnnotation")?,
        cx.builder(),
    );

    Ok(TSType::TSTypeOperatorType(cx.box_in(operator_type)))
}

fn read_ts_mapped_type<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let span: Span = cx.span(node);

    let key: BindingIdentifier<'a> =
        cx.req(node, "key", read_binding_identifier)?;

    let constraint: TSType<'a> = cx.ts(node, "constraint")?;

    let optional: Option<TSMappedTypeModifierOperator> =
        mapped_type_modifier(cx, node.get("optional"))?;

    let readonly: Option<TSMappedTypeModifierOperator> =
        mapped_type_modifier(cx, node.get("readonly"))?;

    let mapped: TSMappedType<'a> = TSMappedType::new(
        span,
        key,
        constraint,
        cx.opt_ts(node, "nameType")?,
        cx.opt_ts(node, "typeAnnotation")?,
        optional,
        readonly,
        cx.builder(),
    );

    Ok(TSType::TSMappedType(cx.box_in(mapped)))
}

fn read_ts_tuple_element<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTupleElement<'a>, ReadError> {
    let ty: &str = ty_of(node);

    if ty == "TSOptionalType" || ty == "TSRestType" {
        let inner: TSType<'a> = cx.ts(node, "typeAnnotation")?;

        return match ty {
            | "TSOptionalType" => {
                let optional: TSOptionalType<'a> =
                    TSOptionalType::new(cx.span(node), inner, cx.builder());
                Ok(TSTupleElement::TSOptionalType(cx.box_in(optional)))
            },
            | _ => {
                let rest: TSRestType<'a> =
                    TSRestType::new(cx.span(node), inner, cx.builder());
                Ok(TSTupleElement::TSRestType(cx.box_in(rest)))
            },
        };
    }

    Ok(TSTupleElement::from(read_ts_type(cx, node)?))
}

pub fn read_ts_type_parameter<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypeParameter<'a>, ReadError> {
    let span: Span = cx.span(node);

    let name: BindingIdentifier<'a> =
        cx.req(node, "name", read_binding_identifier)?;

    let parameter: TSTypeParameter<'a> = TSTypeParameter::new(
        span,
        name,
        cx.opt_ts(node, "constraint")?,
        cx.opt_ts(node, "default")?,
        cx.flag(node, "in"),
        cx.flag(node, "out"),
        cx.flag(node, "const"),
        cx.builder(),
    );

    Ok(parameter)
}

pub fn read_ts_type_parameter_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ArenaBox<'a, TSTypeParameterDeclaration<'a>>, ReadError> {
    Ok(cx.box_in(TSTypeParameterDeclaration::new(
        cx.span(node),
        cx.list(node, "params", read_ts_type_parameter)?,
        cx.builder(),
    )))
}

pub fn read_ts_type_annotation<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Option<ArenaBox<'a, TSTypeAnnotation<'a>>>, ReadError> {
    if node.is_null() {
        return Ok(None);
    }

    let span: Span = cx.span(node);

    let ty: TSType<'a> = cx.req(node, "typeAnnotation", read_ts_type)?;

    Ok(Some(cx.box_in(TSTypeAnnotation::new(span, ty, cx.builder()))))
}

pub fn read_ts_type_parameter_instantiation<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypeParameterInstantiation<'a>, ReadError> {
    Ok(TSTypeParameterInstantiation::new(
        cx.span(node),
        cx.ts_types(node, "params")?,
        cx.builder(),
    ))
}

pub fn read_ts_type<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "TSMappedType" => read_ts_mapped_type(cx, node),
        | "TSTypeOperator" => read_ts_type_operator(cx, node),
        | "TSImportType" => read_ts_import_type(cx, node),
        | "TSFunctionType" => read_ts_function_type(cx, node),
        | "TSConstructorType" => read_ts_constructor_type(cx, node),
        | "TSParenthesizedType" => read_ts_parenthesized_type(cx, node),
        | "TSAnyKeyword" => {
            Ok(TSType::new_ts_any_keyword(cx.span(node), cx.builder()))
        },
        | "TSBigIntKeyword" => {
            Ok(TSType::new_ts_big_int_keyword(cx.span(node), cx.builder()))
        },
        | "TSBooleanKeyword" => {
            Ok(TSType::new_ts_boolean_keyword(cx.span(node), cx.builder()))
        },
        | "TSIntrinsicKeyword" => {
            Ok(TSType::new_ts_intrinsic_keyword(cx.span(node), cx.builder()))
        },
        | "TSNeverKeyword" => {
            Ok(TSType::new_ts_never_keyword(cx.span(node), cx.builder()))
        },
        | "TSNullKeyword" => {
            Ok(TSType::new_ts_null_keyword(cx.span(node), cx.builder()))
        },
        | "TSNumberKeyword" => {
            Ok(TSType::new_ts_number_keyword(cx.span(node), cx.builder()))
        },
        | "TSObjectKeyword" => {
            Ok(TSType::new_ts_object_keyword(cx.span(node), cx.builder()))
        },
        | "TSStringKeyword" => {
            Ok(TSType::new_ts_string_keyword(cx.span(node), cx.builder()))
        },
        | "TSSymbolKeyword" => {
            Ok(TSType::new_ts_symbol_keyword(cx.span(node), cx.builder()))
        },
        | "TSThisType" => {
            Ok(TSType::new_ts_this_type(cx.span(node), cx.builder()))
        },
        | "TSUndefinedKeyword" => {
            Ok(TSType::new_ts_undefined_keyword(cx.span(node), cx.builder()))
        },
        | "TSUnknownKeyword" => {
            Ok(TSType::new_ts_unknown_keyword(cx.span(node), cx.builder()))
        },
        | "TSVoidKeyword" => {
            Ok(TSType::new_ts_void_keyword(cx.span(node), cx.builder()))
        },
        | _ => nodes! { cx, node, ty, TSType :
            "TSTypeReference" => TSTypeReference: TSTypeReference::new [
                cx.span(node),
                cx.req(node, "typeName", read_ts_type_name)?,
                cx.opt_type_arguments(node)?,
            ];
            "TSUnionType" => TSUnionType: TSUnionType::new [
                cx.span(node),
                cx.ts_types(node, "types")?,
            ];
            "TSIntersectionType" => TSIntersectionType: TSIntersectionType::new [
                cx.span(node),
                cx.ts_types(node, "types")?,
            ];
            "TSTypeLiteral" => TSTypeLiteral: TSTypeLiteral::new [
                cx.span(node),
                read_ts_signatures(cx, node, "members")?,
            ];
            "TSLiteralType" => TSLiteralType: TSLiteralType::new [
                cx.span(node),
                cx.req(node, "literal", read_ts_literal)?,
            ];
            "TSConditionalType" => TSConditionalType: TSConditionalType::new [
                cx.span(node),
                cx.ts(node, "checkType")?,
                cx.ts(node, "extendsType")?,
                cx.ts(node, "trueType")?,
                cx.ts(node, "falseType")?,
            ];
            "TSInferType" => TSInferType: TSInferType::new [
                cx.span(node),
                cx.req(node, "typeParameter", |cx: &Cx<'a>, parameter_node: &Value| {
                    Ok(cx.box_in(read_ts_type_parameter(cx, parameter_node)?))
                })?,
            ];
            "TSTupleType" => TSTupleType: TSTupleType::new [
                cx.span(node),
                cx.list(node, "elementTypes", read_ts_tuple_element)?,
            ];
            "TSNamedTupleMember" => TSNamedTupleMember: TSNamedTupleMember::new [
                cx.span(node),
                cx.req(node, "label", read_identifier_name)?,
                cx.req(node, "elementType", read_ts_tuple_element)?,
                cx.flag(node, "optional"),
            ];
            "TSArrayType" => TSArrayType: TSArrayType::new [
                cx.span(node),
                cx.ts(node, "elementType")?,
            ];
            "TSIndexedAccessType" => TSIndexedAccessType: TSIndexedAccessType::new [
                cx.span(node),
                cx.ts(node, "objectType")?,
                cx.ts(node, "indexType")?,
            ];
            "TSTypeQuery" => TSTypeQuery: TSTypeQuery::new [
                cx.span(node),
                cx.req(node, "exprName", read_ts_type_query_expr_name)?,
                cx.opt_type_arguments(node)?,
            ];
            "TSTypePredicate" => TSTypePredicate: TSTypePredicate::new [
                cx.span(node),
                cx.req(node, "parameterName", read_ts_type_predicate_name)?,
                cx.flag(node, "asserts"),
                cx.annotation(node, "typeAnnotation")?,
            ];
            "TSTemplateLiteralType" => TSTemplateLiteralType: TSTemplateLiteralType::new [
                cx.span(node),
                cx.list(node, "quasis", templates::read_template_element)?,
                cx.ts_types(node, "types")?,
            ];
        },
    }
}
