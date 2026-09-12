use crate::reader::json::Value;
use oxc::allocator::Box as ArenaBox;
use oxc::ast::ast::{
    BindingIdentifier, Expression, FormalParameterKind, FormalParameters,
    IdentifierName, IdentifierReference, ObjectExpression, PropertyKey,
    StringLiteral, TSArrayType, TSConditionalType, TSFunctionType,
    TSImportType, TSImportTypeQualifiedName, TSImportTypeQualifier,
    TSIndexSignature, TSIndexSignatureName, TSIndexedAccessType, TSInferType,
    TSIntersectionType, TSLiteral, TSLiteralType, TSMappedType,
    TSMappedTypeModifierOperator, TSMethodSignature, TSMethodSignatureKind,
    TSNamedTupleMember, TSOptionalType, TSPropertySignature, TSQualifiedName,
    TSRestType, TSSignature, TSTemplateLiteralType, TSThisParameter,
    TSThisType, TSTupleElement, TSTupleType, TSType, TSTypeAnnotation,
    TSTypeLiteral, TSTypeName, TSTypeOperator, TSTypeOperatorOperator,
    TSTypeParameter, TSTypeParameterDeclaration, TSTypeParameterInstantiation,
    TSTypePredicate, TSTypePredicateName, TSTypeQuery, TSTypeQueryExprName,
    TSTypeReference, TSUnionType,
};
use oxc::span::Span;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use super::declaration;
use super::engine::{Cx, Seg, nodes, ty_of};
use super::expression;
use crate::errors::read::ReadError;

pub(crate) fn read_ts_type<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "TSMappedType" => read_ts_mapped_type(cx, node),
        | "TSTypeOperator" => read_ts_type_operator(cx, node),
        | "TSImportType" => read_ts_import_type(cx, node),
        | "TSFunctionType" => read_ts_function_type(cx, node),
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
                cx.opt_annotation(node)?,
            ];
            "TSTemplateLiteralType" => TSTemplateLiteralType: TSTemplateLiteralType::new [
                cx.span(node),
                cx.list(node, "quasis", expression::read_template_element)?,
                cx.ts_types(node, "types")?,
            ];
        },
    }
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
                return Err(ReadError::from_message(format!(
                    "unsupported type operator `{}` at {}",
                    other.unwrap_or("<missing>"),
                    cx.path_string()
                )));
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

fn mapped_type_modifier(
    cx: &Cx<'_>,
    value: Option<&Value>,
) -> Result<Option<TSMappedTypeModifierOperator>, ReadError> {
    let modifier: Option<TSMappedTypeModifierOperator> = match value {
        | None => None,
        | Some(v) if v.is_null() => None,
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
            return Err(ReadError::from_message(format!(
                "unsupported mapped type modifier `{other}` at {}",
                cx.path_string()
            )));
        },
    };

    Ok(modifier)
}

pub(crate) fn read_ts_type_name<'a>(
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

pub(crate) fn read_ts_type_name_from_expression<'a>(
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

pub(crate) fn read_ts_type_parameter_instantiation<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypeParameterInstantiation<'a>, ReadError> {
    Ok(TSTypeParameterInstantiation::new(
        cx.span(node),
        cx.ts_types(node, "params")?,
        cx.builder(),
    ))
}

pub(crate) fn read_ts_type_annotation<'a>(
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

pub(crate) fn read_ts_type_parameter_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ArenaBox<'a, TSTypeParameterDeclaration<'a>>, ReadError> {
    Ok(cx.box_in(TSTypeParameterDeclaration::new(
        cx.span(node),
        cx.list(node, "params", read_ts_type_parameter)?,
        cx.builder(),
    )))
}

pub(crate) fn read_ts_type_parameter<'a>(
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

pub(crate) fn read_binding_identifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingIdentifier<'a>, ReadError> {
    Ok(BindingIdentifier::new(cx.span(node), cx.name(node)?, cx.builder()))
}

pub(crate) fn read_identifier_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<IdentifierName<'a>, ReadError> {
    Ok(IdentifierName::new(cx.span(node), cx.name(node)?, cx.builder()))
}

fn read_ts_literal<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSLiteral<'a>, ReadError> {
    let literal_node: TSLiteral<'a> =
        match expression::read_expression(cx, node)? {
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

fn read_ts_type_query_expr_name<'a>(
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
        | "Identifier" | "TSQualifiedName" => {
            let name: TSTypeName<'a> = read_ts_type_name(cx, node)?;
            Ok(TSTypeQueryExprName::from(name))
        },
        | _ => Err(cx.err(node)),
    }
}

fn read_ts_import_type<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let span: Span = cx.span(node);

    let source: StringLiteral<'a> = cx.string_literal(node, "source")?;

    let options: Option<ArenaBox<'a, ObjectExpression<'a>>> =
        cx.opt(node, "options", |cx: &Cx<'a>, options_node: &Value| {
            match expression::read_expression(cx, options_node)? {
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

fn read_ts_type_predicate_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypePredicateName<'a>, ReadError> {
    match ty_of(node) {
        | "Identifier" => Ok(TSTypePredicateName::Identifier(
            cx.box_in(read_identifier_name(cx, node)?),
        )),
        | "TSThisParameter" => {
            let this: TSThisType = TSThisType::new(cx.span(node), cx.builder());
            Ok(TSTypePredicateName::This(cx.box_in(this)))
        },
        | _ => Err(cx.err(node)),
    }
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

pub(crate) fn read_ts_signatures<'a>(
    cx: &Cx<'a>,
    node: &Value,
    field: &'static str,
) -> Result<oxc::allocator::Vec<'a, TSSignature<'a>>, ReadError> {
    cx.list(node, field, read_ts_signature)
}

fn read_ts_signature<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSSignature<'a>, ReadError> {
    match ty_of(node) {
        | "TSPropertySignature" => {
            let key: PropertyKey<'a> = cx.property_key(node, "key")?;

            let signature: TSPropertySignature<'a> = TSPropertySignature::new(
                cx.span(node),
                cx.flag(node, "computed"),
                cx.flag(node, "optional"),
                cx.flag(node, "readonly"),
                key,
                cx.opt_annotation(node)?,
                cx.builder(),
            );

            Ok(TSSignature::TSPropertySignature(cx.box_in(signature)))
        },
        | "TSMethodSignature" => {
            let span: Span = cx.span(node);

            let key: PropertyKey<'a> = cx.property_key(node, "key")?;

            let kind: TSMethodSignatureKind =
                match node.get("kind").and_then(Value::as_str) {
                    | Some("method") => TSMethodSignatureKind::Method,
                    | Some("get") => TSMethodSignatureKind::Get,
                    | Some("set") => TSMethodSignatureKind::Set,
                    | other => {
                        return Err(ReadError::from_message(format!(
                            "unsupported method signature kind `{}` at {}",
                            other.unwrap_or("<missing>"),
                            cx.path_string()
                        )));
                    },
                };

            let (params, this_param) = signature_params(cx, node, span)?;

            let signature: TSMethodSignature<'a> = TSMethodSignature::new(
                span,
                key,
                cx.flag(node, "computed"),
                cx.flag(node, "optional"),
                kind,
                cx.opt_type_parameters(node)?,
                this_param,
                cx.box_in(params),
                cx.annotation(node, "returnType")?,
                cx.builder(),
            );

            Ok(TSSignature::TSMethodSignature(cx.box_in(signature)))
        },
        | "TSIndexSignature" => {
            let span: Span = cx.span(node);

            let parameter_node: &Value = node
                .get("parameters")
                .and_then(Value::as_array)
                .and_then(|params: &sonic_rs::Array| params.first())
                .ok_or_else(|| cx.err(node))?;

            let parameter: TSIndexSignatureName<'a> =
                cx.child(Seg::field("parameters"), |cx| {
                    cx.child(Seg::index(0), |cx| {
                        read_ts_index_signature_name(cx, parameter_node)
                    })
                })?;

            let annotation_node: &Value =
                node.get("typeAnnotation").ok_or_else(|| cx.err(node))?;

            let type_annotation: ArenaBox<'a, TSTypeAnnotation<'a>> = cx
                .child(Seg::field("typeAnnotation"), |cx| {
                    read_ts_type_annotation(cx, annotation_node)?
                        .ok_or_else(|| cx.err(annotation_node))
                })?;

            let signature: TSIndexSignature<'a> = TSIndexSignature::new(
                span,
                parameter,
                type_annotation,
                cx.flag(node, "readonly"),
                cx.flag(node, "static"),
                cx.builder(),
            );

            Ok(TSSignature::TSIndexSignature(cx.box_in(signature)))
        },
        | _ => Err(cx.err(node)),
    }
}

fn read_ts_index_signature_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSIndexSignatureName<'a>, ReadError> {
    let span: Span = cx.span(node);

    let name: oxc::str::Ident<'a> = cx.name(node)?;

    let type_annotation: ArenaBox<'a, TSTypeAnnotation<'a>> =
        cx.opt_annotation(node)?.ok_or_else(|| cx.err(node))?;

    Ok(TSIndexSignatureName::new(span, name, type_annotation, cx.builder()))
}

fn signature_params<'a>(
    cx: &Cx<'a>,
    node: &Value,
    fallback_span: Span,
) -> Result<
    (FormalParameters<'a>, Option<ArenaBox<'a, TSThisParameter<'a>>>),
    ReadError,
> {
    let params_node: &Value = node.get("params").ok_or_else(|| cx.err(node))?;

    let empty: [Value; 0] = [];

    let items: &[Value] = params_node
        .as_array()
        .map_or(&empty, |params: &sonic_rs::Array| params.as_slice());

    declaration::params_in(
        cx,
        items,
        fallback_span,
        FormalParameterKind::FormalParameter,
    )
}

fn read_ts_function_type<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let span: Span = cx.span(node);

    let (params, this_param) = signature_params(cx, node, span)?;

    let return_node: &Value =
        node.get("returnType").ok_or_else(|| cx.err(node))?;

    let return_type: ArenaBox<'a, TSTypeAnnotation<'a>> =
        cx.child(Seg::field("returnType"), |cx| {
            read_ts_type_annotation(cx, return_node)?
                .ok_or_else(|| cx.err(return_node))
        })?;

    let function: TSFunctionType<'a> = TSFunctionType::new(
        span,
        cx.opt_type_parameters(node)?,
        this_param,
        cx.box_in(params),
        return_type,
        cx.builder(),
    );

    Ok(TSType::TSFunctionType(cx.box_in(function)))
}
