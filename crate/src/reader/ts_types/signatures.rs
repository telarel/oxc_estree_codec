use oxc::allocator::Box as ArenaBox;
use oxc::ast::ast::{
    FormalParameterKind, FormalParameters, PropertyKey,
    TSCallSignatureDeclaration, TSConstructSignatureDeclaration,
    TSConstructorType, TSFunctionType, TSIndexSignature, TSIndexSignatureName,
    TSMethodSignature, TSMethodSignatureKind, TSParenthesizedType,
    TSPropertySignature, TSSignature, TSThisParameter, TSThisType, TSType,
    TSTypeAnnotation, TSTypePredicateName,
};
use oxc::span::Span;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use crate::errors::read::ReadError;
use crate::reader::declaration::function;
use crate::reader::engine::context::{Cx, Seg, ty_of};
use crate::reader::json::Value;
use crate::reader::ts_types::names::read_identifier_name;
use crate::reader::ts_types::types::read_ts_type_annotation;

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

    function::params_in(
        cx,
        items,
        fallback_span,
        FormalParameterKind::FormalParameter,
    )
}

pub fn read_ts_index_signature_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSIndexSignatureName<'a>, ReadError> {
    let span: Span = cx.span(node);

    let name: oxc::str::Ident<'a> = cx.name(node)?;

    let type_annotation: ArenaBox<'a, TSTypeAnnotation<'a>> =
        cx.annotation(node, "typeAnnotation")?.ok_or_else(|| cx.err(node))?;

    Ok(TSIndexSignatureName::new(span, name, type_annotation, cx.builder()))
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
                cx.annotation(node, "typeAnnotation")?,
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
                        return Err(ReadError::ValueUnsupported {
                            kind: "method signature kind",
                            value: other.unwrap_or("<missing>").to_string(),
                            path: cx.path_string(),
                        });
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
        | "TSCallSignatureDeclaration" | "TSConstructSignatureDeclaration" => {
            let span: Span = cx.span(node);

            let (params, this_param) = signature_params(cx, node, span)?;

            let return_type: Option<ArenaBox<'a, TSTypeAnnotation<'a>>> =
                cx.annotation(node, "returnType")?;

            // the serialized `params` array includes `this` as its first
            // element for call signatures (see oxc's
            // `TSCallSignatureDeclarationParams` serializer); it is read
            // back out into `this_param` here.
            let signature: TSSignature<'a> =
                if ty_of(node) == "TSCallSignatureDeclaration" {
                    TSSignature::TSCallSignatureDeclaration(cx.box_in(
                        TSCallSignatureDeclaration::new(
                            span,
                            cx.opt_type_parameters(node)?,
                            this_param,
                            cx.box_in(params),
                            return_type,
                            cx.builder(),
                        ),
                    ))
                } else {
                    TSSignature::TSConstructSignatureDeclaration(cx.box_in(
                        TSConstructSignatureDeclaration::new(
                            span,
                            cx.opt_type_parameters(node)?,
                            cx.box_in(params),
                            return_type,
                            cx.builder(),
                        ),
                    ))
                };

            Ok(signature)
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

pub fn read_ts_function_type<'a>(
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

pub fn read_ts_constructor_type<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let span: Span = cx.span(node);

    let (params, this_param) = signature_params(cx, node, span)?;

    // a `this` parameter is not valid on a constructor type, but reading
    // through the shared signature params keeps paths uniform; the
    // serialized `params` array does not include `this` here.
    debug_assert!(this_param.is_none());

    let return_type: ArenaBox<'a, TSTypeAnnotation<'a>> =
        cx.child(Seg::field("returnType"), |cx| {
            // the serialized `returnType` is the `TSTypeAnnotation` node
            // itself, not a wrapper with an inner `typeAnnotation`.
            cx.req(node, "returnType", read_ts_type_annotation)?
                .ok_or_else(|| cx.err(node))
        })?;

    let constructor: TSConstructorType<'a> = TSConstructorType::new(
        span,
        cx.flag(node, "abstract"),
        cx.opt_type_parameters(node)?,
        cx.box_in(params),
        return_type,
        cx.builder(),
    );

    Ok(TSType::TSConstructorType(cx.box_in(constructor)))
}

pub fn read_ts_parenthesized_type<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSType<'a>, ReadError> {
    let paren: TSParenthesizedType<'a> = TSParenthesizedType::new(
        cx.span(node),
        cx.ts(node, "typeAnnotation")?,
        cx.builder(),
    );

    Ok(TSType::TSParenthesizedType(cx.box_in(paren)))
}

pub fn read_ts_signatures<'a>(
    cx: &Cx<'a>,
    node: &Value,
    field: &'static str,
) -> Result<oxc::allocator::Vec<'a, TSSignature<'a>>, ReadError> {
    cx.list(node, field, read_ts_signature)
}

pub fn read_ts_type_predicate_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypePredicateName<'a>, ReadError> {
    match ty_of(node) {
        | "Identifier" => Ok(TSTypePredicateName::Identifier(
            cx.box_in(read_identifier_name(cx, node)?),
        )),
        | "TSThisType" => {
            let this: TSThisType = TSThisType::new(cx.span(node), cx.builder());
            Ok(TSTypePredicateName::This(cx.box_in(this)))
        },
        | _ => Err(cx.err(node)),
    }
}
