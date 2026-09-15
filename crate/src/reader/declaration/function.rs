use oxc::ast::ast::{
    BindingIdentifier, BindingPattern, BindingRestElement, Decorator,
    Expression, FormalParameter, FormalParameterKind, FormalParameterRest,
    FormalParameters, Function, FunctionBody, FunctionType, TSAccessibility,
    TSThisParameter, TSTypeAnnotation,
};
use oxc::span::Span;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, Seg, ty_of};
use crate::reader::engine::program::read_function_body;
use crate::reader::expression::expressions;
use crate::reader::json::Value;
use crate::reader::pattern;

pub fn accessibility<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Option<TSAccessibility>, ReadError> {
    match node.get("accessibility").and_then(Value::as_str) {
        | None | Some("null") => Ok(None),
        | Some("private") => Ok(Some(TSAccessibility::Private)),
        | Some("protected") => Ok(Some(TSAccessibility::Protected)),
        | Some("public") => Ok(Some(TSAccessibility::Public)),
        | Some(other) => Err(ReadError::ValueUnsupported {
            kind: "accessibility",
            value: other.to_string(),
            path: cx.path_string(),
        }),
    }
}

pub fn read_binding_identifier_at<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingIdentifier<'a>, ReadError> {
    Ok(BindingIdentifier::new(cx.span(node), cx.name(node)?, cx.builder()))
}

pub fn read_decorators<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<oxc::allocator::Vec<'a, Decorator<'a>>, ReadError> {
    cx.list(node, "decorators", |cx, item| {
        let expression_node: &Value =
            item.get("expression").ok_or_else(|| cx.err(item))?;

        let expression: Expression<'a> = cx
            .child(Seg::field("expression"), |cx| {
                expressions::read_expression(cx, expression_node)
            })?;

        Ok(Decorator::new(cx.span(item), expression, cx.builder()))
    })
}

fn formal_parameter<'a>(
    cx: &Cx<'a>,
    item: &Value,
) -> Result<FormalParameter<'a>, ReadError> {
    let decorators: oxc::allocator::Vec<'a, Decorator<'a>> =
        read_decorators(cx, item)?;

    let param_span: Span = cx.span(item);

    let param_ty: &str = ty_of(item);

    if param_ty == "TSParameterProperty" {
        return cx.child(Seg::field("parameter"), |cx| {
            let parameter_node: &Value =
                item.get("parameter").ok_or_else(|| cx.err(item))?;

            let accessibility: Option<TSAccessibility> =
                accessibility(cx, item)?;

            let readonly: bool = cx.flag(item, "readonly");

            let is_override: bool = cx.flag(item, "override");

            let optional: bool = cx.flag(parameter_node, "optional");

            if ty_of(parameter_node) == "AssignmentPattern" {
                let left_node: &Value = parameter_node
                    .get("left")
                    .ok_or_else(|| cx.err(parameter_node))?;

                let (pattern, type_annotation): (
                    BindingPattern<'a>,
                    Option<oxc::allocator::Box<'a, TSTypeAnnotation<'a>>>,
                ) = cx.child(Seg::field("left"), |cx| {
                    let pattern: BindingPattern<'a> =
                        pattern::binding::read_binding_pattern(cx, left_node)?;

                    let type_annotation =
                        cx.annotation(left_node, "typeAnnotation")?;

                    Ok((pattern, type_annotation))
                })?;

                let param: FormalParameter<'a> = FormalParameter::new(
                    param_span,
                    decorators,
                    pattern,
                    type_annotation,
                    Some(cx.box_in(cx.expr(parameter_node, "right")?)),
                    false,
                    accessibility,
                    readonly,
                    is_override,
                    cx.builder(),
                );

                return Ok(param);
            }

            let pattern: BindingPattern<'a> =
                pattern::binding::read_binding_pattern(cx, parameter_node)?;

            let type_annotation =
                cx.annotation(parameter_node, "typeAnnotation")?;

            let param: FormalParameter<'a> = FormalParameter::new(
                param_span,
                decorators,
                pattern,
                type_annotation,
                None,
                optional,
                accessibility,
                readonly,
                is_override,
                cx.builder(),
            );

            Ok(param)
        });
    }

    let optional: bool = cx.flag(item, "optional");

    if param_ty == "AssignmentPattern" {
        let left_node: &Value = item.get("left").ok_or_else(|| cx.err(item))?;

        let (pattern, type_annotation): (
            BindingPattern<'a>,
            Option<oxc::allocator::Box<'a, TSTypeAnnotation<'a>>>,
        ) = cx.child(Seg::field("left"), |cx| {
            let pattern: BindingPattern<'a> =
                pattern::binding::read_binding_pattern(cx, left_node)?;

            let type_annotation = cx.annotation(left_node, "typeAnnotation")?;

            Ok((pattern, type_annotation))
        })?;

        let param: FormalParameter<'a> = FormalParameter::new(
            param_span,
            decorators,
            pattern,
            type_annotation,
            Some(cx.box_in(cx.expr(item, "right")?)),
            optional,
            None,
            false,
            false,
            cx.builder(),
        );

        return Ok(param);
    }

    let pattern: BindingPattern<'a> =
        pattern::binding::read_binding_pattern(cx, item)?;

    let type_annotation = cx.annotation(item, "typeAnnotation")?;

    let param: FormalParameter<'a> = FormalParameter::new(
        param_span,
        decorators,
        pattern,
        type_annotation,
        None,
        optional,
        None,
        false,
        false,
        cx.builder(),
    );

    Ok(param)
}

pub fn params_in<'a>(
    cx: &Cx<'a>,
    items: &[Value],
    span: Span,
    kind: FormalParameterKind,
) -> Result<
    (
        FormalParameters<'a>,
        Option<oxc::allocator::Box<'a, TSThisParameter<'a>>>,
    ),
    ReadError,
> {
    let mut parameters: oxc::allocator::Vec<'a, FormalParameter<'a>> =
        oxc::allocator::Vec::new_in(cx.builder());

    let mut rest: Option<oxc::allocator::Box<'a, FormalParameterRest<'a>>> =
        None;

    let mut this_param: Option<oxc::allocator::Box<'a, TSThisParameter<'a>>> =
        None;

    cx.push(Seg::field("params"));

    for (index, item) in items.iter().enumerate() {
        cx.push(Seg::index(index));

        let step: Result<(), ReadError> = (|| {
            let item_ty: &str = ty_of(item);

            if item_ty == "RestElement" {
                let argument: BindingPattern<'a> = cx.req(
                    item,
                    "argument",
                    pattern::binding::read_binding_pattern,
                )?;

                let rest_element: BindingRestElement<'a> =
                    BindingRestElement::new(
                        cx.span(item),
                        argument,
                        cx.builder(),
                    );

                let param_rest: FormalParameterRest<'a> =
                    FormalParameterRest::new(
                        cx.span(item),
                        read_decorators(cx, item)?,
                        rest_element,
                        cx.annotation(item, "typeAnnotation")?,
                        cx.builder(),
                    );

                rest = Some(cx.box_in(param_rest));

                return Ok(());
            }

            if item_ty == "Identifier"
                && item.get("name").and_then(Value::as_str) == Some("this")
            {
                let param_span: Span = cx.span(item);

                let this: TSThisParameter<'a> = TSThisParameter::new(
                    param_span,
                    param_span,
                    cx.annotation(item, "typeAnnotation")?,
                    cx.builder(),
                );

                this_param = Some(cx.box_in(this));

                return Ok(());
            }

            parameters.push(formal_parameter(cx, item)?);

            Ok(())
        })();

        cx.pop();

        step?;
    }

    cx.pop();

    let params: FormalParameters =
        FormalParameters::new(span, kind, parameters, rest, cx.builder());

    Ok((params, this_param))
}

pub fn read_formal_parameters<'a>(
    cx: &Cx<'a>,
    node: &Value,
    fallback_span: Span,
    kind: FormalParameterKind,
) -> Result<
    (
        FormalParameters<'a>,
        Option<oxc::allocator::Box<'a, TSThisParameter<'a>>>,
    ),
    ReadError,
> {
    let span: Span = match node.get("start") {
        | Some(_) => cx.span(node),
        | None => fallback_span,
    };

    let empty: [Value; 0] = [];

    let items: &[Value] = node
        .get("params")
        .and_then(Value::as_array)
        .map_or(&empty, |params: &sonic_rs::Array| params.as_slice());

    params_in(cx, items, span, kind)
}

pub fn read_binding_identifier_field<'a>(
    cx: &Cx<'a>,
    node: &Value,
    field: &'static str,
) -> Result<BindingIdentifier<'a>, ReadError> {
    cx.req(node, field, read_binding_identifier_at)
}

pub fn read_function<'a>(
    cx: &Cx<'a>,
    node: &Value,
    function_type: FunctionType,
) -> Result<Function<'a>, ReadError> {
    let span: Span = cx.span(node);

    let id: Option<BindingIdentifier<'a>> =
        cx.opt(node, "id", |cx: &Cx<'a>, id_node: &Value| {
            Ok(BindingIdentifier::new(
                cx.span(id_node),
                cx.name(id_node)?,
                cx.builder(),
            ))
        })?;

    let (params, this_param): (
        FormalParameters<'a>,
        Option<oxc::allocator::Box<'a, TSThisParameter<'a>>>,
    ) = read_formal_parameters(
        cx,
        node,
        span,
        FormalParameterKind::FormalParameter,
    )?;

    let body: Option<oxc::allocator::Box<'a, FunctionBody<'a>>> =
        cx.opt(node, "body", |cx: &Cx<'a>, body_node: &Value| {
            let function_body: FunctionBody<'a> =
                read_function_body(cx, body_node)?;
            Ok(cx.box_in(function_body))
        })?;

    let function: Function<'a> = Function::new(
        span,
        function_type,
        id,
        cx.flag(node, "generator"),
        cx.flag(node, "async"),
        cx.flag(node, "declare"),
        cx.opt_type_parameters(node)?,
        this_param,
        cx.box_in(params),
        cx.annotation(node, "returnType")?,
        body,
        cx.builder(),
    );

    Ok(function)
}
