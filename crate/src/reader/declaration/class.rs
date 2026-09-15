use oxc::ast::ast::{
    AccessorProperty, AccessorPropertyType, BindingIdentifier, Class,
    ClassBody, ClassElement, ClassHeritage, ClassType, Function, FunctionType,
    MethodDefinition, MethodDefinitionKind, MethodDefinitionType,
    PropertyDefinition, PropertyDefinitionType, PropertyKey, StaticBlock,
    TSClassImplements, TSIndexSignature, TSIndexSignatureName,
    TSTypeAnnotation, TSTypeName,
};
use oxc::span::Span;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use crate::errors::read::ReadError;
use crate::reader::declaration::function::{
    accessibility, read_decorators, read_function,
};
use crate::reader::engine::context::{Cx, Seg, ty_of};
use crate::reader::expression::expressions;
use crate::reader::json::Value;

fn read_class_implements<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<oxc::allocator::Vec<'a, TSClassImplements<'a>>, ReadError> {
    cx.list(node, "implements", |cx, item| {
        let expression_node: &Value =
            item.get("expression").ok_or_else(|| cx.err(item))?;

        let expression: TSTypeName<'a> =
            cx.child(Seg::field("expression"), |cx| {
                crate::reader::ts_types::names::read_ts_type_name_from_expression(cx, expression_node)
            })?;

        let implements_entry: TSClassImplements<'a> = TSClassImplements::new(
            cx.span(item),
            expression,
            cx.opt_box(
                item,
                "typeArguments",
                crate::reader::ts_types::types::read_ts_type_parameter_instantiation,
            )?,
            cx.builder(),
        );

        Ok(implements_entry)
    })
}

fn method_definition<'a>(
    cx: &Cx<'a>,
    node: &Value,
    ty: &str,
) -> Result<MethodDefinition<'a>, ReadError> {
    let span: Span = cx.span(node);

    let key: PropertyKey<'a> = cx.property_key(node, "key")?;

    let function: Function<'a> = cx.req(node, "value", |cx, value_node| {
        // TS-ESTree represents method/function values without a body as
        // `TSEmptyBodyFunctionExpression`; oxc models that as a `Function`
        // with `FunctionType::TSEmptyBodyFunctionExpression`.
        let value_type: &str = ty_of(value_node);

        let function_type: FunctionType =
            if value_type == "TSEmptyBodyFunctionExpression" {
                FunctionType::TSEmptyBodyFunctionExpression
            } else {
                FunctionType::FunctionExpression
            };

        read_function(cx, value_node, function_type)
    })?;

    let kind: MethodDefinitionKind =
        match node.get("kind").and_then(Value::as_str) {
            | Some("method") => MethodDefinitionKind::Method,
            | Some("get") => MethodDefinitionKind::Get,
            | Some("set") => MethodDefinitionKind::Set,
            | Some("constructor") => MethodDefinitionKind::Constructor,
            | _ => return Err(cx.err(node)),
        };

    let method_type: MethodDefinitionType = match ty {
        | "TSAbstractMethodDefinition" => {
            MethodDefinitionType::TSAbstractMethodDefinition
        },
        | _ => MethodDefinitionType::MethodDefinition,
    };

    let method: MethodDefinition<'a> = MethodDefinition::new(
        span,
        method_type,
        read_decorators(cx, node)?,
        key,
        cx.box_in(function),
        kind,
        cx.flag(node, "computed"),
        cx.flag(node, "static"),
        cx.flag(node, "override"),
        cx.flag(node, "optional"),
        accessibility(cx, node)?,
        cx.builder(),
    );

    Ok(method)
}

fn property_definition<'a>(
    cx: &Cx<'a>,
    node: &Value,
    ty: &str,
) -> Result<PropertyDefinition<'a>, ReadError> {
    let span: Span = cx.span(node);

    let key: PropertyKey<'a> = cx.property_key(node, "key")?;

    let property_type: PropertyDefinitionType = match ty {
        | "TSAbstractPropertyDefinition" => {
            PropertyDefinitionType::TSAbstractPropertyDefinition
        },
        | _ => PropertyDefinitionType::PropertyDefinition,
    };

    let property: PropertyDefinition<'a> = PropertyDefinition::new(
        span,
        property_type,
        read_decorators(cx, node)?,
        key,
        cx.annotation(node, "typeAnnotation")?,
        cx.opt_expr(node, "value")?,
        cx.flag(node, "computed"),
        cx.flag(node, "static"),
        cx.flag(node, "declare"),
        cx.flag(node, "override"),
        cx.flag(node, "optional"),
        cx.flag(node, "definite"),
        cx.flag(node, "readonly"),
        accessibility(cx, node)?,
        cx.builder(),
    );

    Ok(property)
}

fn accessor_property<'a>(
    cx: &Cx<'a>,
    node: &Value,
    ty: &str,
) -> Result<AccessorProperty<'a>, ReadError> {
    let span: Span = cx.span(node);

    let key: PropertyKey<'a> = cx.property_key(node, "key")?;

    let accessor_type: AccessorPropertyType = match ty {
        | "TSAbstractAccessorProperty" => {
            AccessorPropertyType::TSAbstractAccessorProperty
        },
        | _ => AccessorPropertyType::AccessorProperty,
    };

    let property: AccessorProperty<'a> = AccessorProperty::new(
        span,
        accessor_type,
        read_decorators(cx, node)?,
        key,
        cx.annotation(node, "typeAnnotation")?,
        cx.opt_expr(node, "value")?,
        cx.flag(node, "computed"),
        cx.flag(node, "static"),
        cx.flag(node, "override"),
        cx.flag(node, "definite"),
        accessibility(cx, node)?,
        cx.builder(),
    );

    Ok(property)
}

fn class_element<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ClassElement<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "StaticBlock" => {
            let block: StaticBlock<'a> = StaticBlock::new(
                cx.span(node),
                cx.stmts(node, "body")?,
                cx.builder(),
            );
            Ok(ClassElement::StaticBlock(cx.box_in(block)))
        },
        | "MethodDefinition" | "TSAbstractMethodDefinition" => {
            Ok(ClassElement::MethodDefinition(
                cx.box_in(method_definition(cx, node, ty)?),
            ))
        },
        | "PropertyDefinition" | "TSAbstractPropertyDefinition" => {
            Ok(ClassElement::PropertyDefinition(
                cx.box_in(property_definition(cx, node, ty)?),
            ))
        },
        | "AccessorProperty" | "TSAbstractAccessorProperty" => {
            Ok(ClassElement::AccessorProperty(
                cx.box_in(accessor_property(cx, node, ty)?),
            ))
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
                        crate::reader::ts_types::signatures::read_ts_index_signature_name(
                            cx,
                            parameter_node,
                        )
                    })
                })?;

            let annotation_node: &Value =
                node.get("typeAnnotation").ok_or_else(|| cx.err(node))?;

            let type_annotation: oxc::allocator::Box<'a, TSTypeAnnotation<'a>> =
                cx.child(Seg::field("typeAnnotation"), |cx| {
                    crate::reader::ts_types::types::read_ts_type_annotation(
                        cx,
                        annotation_node,
                    )?
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

            Ok(ClassElement::TSIndexSignature(cx.box_in(signature)))
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_class<'a>(
    cx: &Cx<'a>,
    node: &Value,
    class_type: ClassType,
) -> Result<Class<'a>, ReadError> {
    let span: Span = cx.span(node);

    let id: Option<BindingIdentifier<'a>> =
        cx.opt(node, "id", |cx: &Cx<'a>, id_node: &Value| {
            Ok(BindingIdentifier::new(
                cx.span(id_node),
                cx.name(id_node)?,
                cx.builder(),
            ))
        })?;

    let heritage: Option<ClassHeritage<'a>> =
        cx.opt(node, "superClass", |cx: &Cx<'a>, super_node: &Value| {
            Ok(ClassHeritage {
                expression: expressions::read_expression(cx, super_node)?,
                type_arguments: cx.opt_box(
                    node,
                    "superTypeArguments",
                    crate::reader::ts_types::types::read_ts_type_parameter_instantiation,
                )?,
            })
        })?;

    let body_node: Option<&Value> = node.get("body");

    let elements: oxc::allocator::Vec<'a, ClassElement<'a>> = match body_node {
        | Some(body) => cx.child(Seg::field("body"), |cx| {
            cx.list(body, "body", class_element)
        }),
        | None => Ok(oxc::allocator::Vec::new_in(cx.builder())),
    }?;

    let body: ClassBody<'a> = ClassBody::new(
        body_node.map(|b| cx.span(b)).unwrap_or(span),
        elements,
        cx.builder(),
    );

    let class: Class<'a> = Class::new(
        span,
        class_type,
        read_decorators(cx, node)?,
        id,
        cx.opt_type_parameters(node)?,
        heritage,
        read_class_implements(cx, node)?,
        cx.box_in(body),
        cx.flag(node, "abstract"),
        cx.flag(node, "declare"),
        cx.builder(),
    );

    Ok(class)
}

pub fn read_class_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Class<'a>, ReadError> {
    read_class(cx, node, ClassType::ClassDeclaration)
}
