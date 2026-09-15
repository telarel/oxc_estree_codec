use oxc::allocator::{Box as ArenaBox, Vec as ArenaVec};
use oxc::ast::ast::{
    ArrayAssignmentTarget, AssignmentTarget, AssignmentTargetMaybeDefault,
    AssignmentTargetPattern, AssignmentTargetProperty,
    AssignmentTargetPropertyIdentifier, AssignmentTargetPropertyProperty,
    AssignmentTargetRest, AssignmentTargetWithDefault, Expression,
    IdentifierReference, MemberExpression, ObjectAssignmentTarget,
    PrivateIdentifier, PropertyKey, SimpleAssignmentTarget,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, ty_of};
use crate::reader::expression::expressions::read_expression;
use crate::reader::json::Value;

fn expression_to_assignment_target<'a>(
    cx: &Cx<'a>,
    expression: Expression<'a>,
    node: &Value,
) -> Result<AssignmentTarget<'a>, ReadError> {
    match expression {
        | Expression::Identifier(ident) => Ok(AssignmentTarget::from(
            SimpleAssignmentTarget::AssignmentTargetIdentifier(ident),
        )),
        | expr @ (Expression::ComputedMemberExpression(_)
        | Expression::StaticMemberExpression(_)
        | Expression::PrivateFieldExpression(_)) => {
            let member: MemberExpression<'a> =
                match MemberExpression::try_from(expr) {
                    | Ok(member) => member,
                    | Err(_) => return Err(cx.err(node)),
                };
            Ok(AssignmentTarget::from(member))
        },
        // TS-ESTree allows `x as T` / `x!` / `<T>x` as assignment targets
        // (e.g. `for ((x as T) in y)`); oxc models them as
        // `SimpleAssignmentTarget` variants.
        | Expression::TSAsExpression(inner) => Ok(AssignmentTarget::from(
            SimpleAssignmentTarget::TSAsExpression(inner),
        )),
        | Expression::TSSatisfiesExpression(inner) => {
            Ok(AssignmentTarget::from(
                SimpleAssignmentTarget::TSSatisfiesExpression(inner),
            ))
        },
        | Expression::TSNonNullExpression(inner) => Ok(AssignmentTarget::from(
            SimpleAssignmentTarget::TSNonNullExpression(inner),
        )),
        | Expression::TSTypeAssertion(inner) => Ok(AssignmentTarget::from(
            SimpleAssignmentTarget::TSTypeAssertion(inner),
        )),
        | _ => Err(cx.err(node)),
    }
}

fn array_assignment_target<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<AssignmentTarget<'a>, ReadError> {
    let span: Span = cx.span(node);

    let mut elements: ArenaVec<'a, Option<AssignmentTargetMaybeDefault<'a>>> =
        ArenaVec::new_in(cx.builder());

    let mut rest: Option<ArenaBox<'a, AssignmentTargetRest<'a>>> = None;

    cx.list(node, "elements", |cx, item| {
        if ty_of(item) == "RestElement" {
            if rest.is_some() {
                return Err(cx.err(item));
            }

            let target: AssignmentTarget<'a> =
                cx.req(item, "argument", read_assignment_target)?;

            let rest_element: AssignmentTargetRest<'a> =
                AssignmentTargetRest::new(cx.span(item), target, cx.builder());

            rest = Some(cx.box_in(rest_element));

            return Ok(());
        }

        if item.is_null() {
            elements.push(None);

            return Ok(());
        }

        elements.push(Some(read_assignment_target_maybe_default(cx, item)?));

        Ok(())
    })?;

    let target: ArrayAssignmentTarget<'a> =
        ArrayAssignmentTarget::new(span, elements, rest, cx.builder());

    Ok(AssignmentTarget::from(AssignmentTargetPattern::ArrayAssignmentTarget(
        cx.box_in(target),
    )))
}

fn assignment_target_property<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<AssignmentTargetProperty<'a>, ReadError> {
    if cx.flag(node, "shorthand") {
        let key_node: &Value = node.get("key").ok_or_else(|| cx.err(node))?;

        let value_node: &Value =
            node.get("value").ok_or_else(|| cx.err(node))?;

        let value_ty: &str = ty_of(value_node);

        let (binding_node, init): (&Value, Option<Expression<'a>>) =
            if value_ty == "AssignmentPattern" {
                let init: Expression<'a> =
                    cx.req(value_node, "right", |cx, init_node| {
                        read_expression(cx, init_node)
                    })?;

                let left_node: &Value =
                    value_node.get("left").ok_or_else(|| cx.err(value_node))?;

                (left_node, Some(init))
            } else {
                (key_node, None)
            };

        let binding_span: Span = cx.span(binding_node);

        let name: oxc::str::Ident<'a> = cx.name(binding_node)?;

        let binding: IdentifierReference<'a> =
            IdentifierReference::new(binding_span, name, cx.builder());

        let property: AssignmentTargetPropertyIdentifier<'a> =
            AssignmentTargetPropertyIdentifier::new(
                cx.span(node),
                binding,
                init,
                cx.builder(),
            );

        return Ok(
            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
                cx.box_in(property),
            ),
        );
    }

    let property: AssignmentTargetPropertyProperty<'a> =
        AssignmentTargetPropertyProperty::new(
            cx.span(node),
            cx.property_key(node, "key")?,
            cx.req(node, "value", read_assignment_target_maybe_default)?,
            cx.flag(node, "computed"),
            cx.builder(),
        );

    Ok(AssignmentTargetProperty::AssignmentTargetPropertyProperty(
        cx.box_in(property),
    ))
}

fn object_assignment_target<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<AssignmentTarget<'a>, ReadError> {
    let span: Span = cx.span(node);

    let mut properties: ArenaVec<'a, AssignmentTargetProperty<'a>> =
        ArenaVec::new_in(cx.builder());

    let mut rest: Option<ArenaBox<'a, AssignmentTargetRest<'a>>> = None;

    cx.list(node, "properties", |cx, item| {
        if ty_of(item) == "RestElement" {
            if rest.is_some() {
                return Err(cx.err(item));
            }

            let target: AssignmentTarget<'a> =
                cx.req(item, "argument", read_assignment_target)?;

            let rest_element: AssignmentTargetRest<'a> =
                AssignmentTargetRest::new(cx.span(item), target, cx.builder());

            rest = Some(cx.box_in(rest_element));

            return Ok(());
        }

        properties.push(assignment_target_property(cx, item)?);

        Ok(())
    })?;

    let target: ObjectAssignmentTarget<'a> =
        ObjectAssignmentTarget::new(span, properties, rest, cx.builder());

    Ok(AssignmentTarget::from(AssignmentTargetPattern::ObjectAssignmentTarget(
        cx.box_in(target),
    )))
}

fn read_assignment_target_maybe_default<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<AssignmentTargetMaybeDefault<'a>, ReadError> {
    if ty_of(node) == "AssignmentPattern" {
        let with_default: AssignmentTargetWithDefault<'a> =
            AssignmentTargetWithDefault::new(
                cx.span(node),
                cx.req(node, "left", read_assignment_target)?,
                cx.expr(node, "right")?,
                cx.builder(),
            );

        return Ok(AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(
            cx.box_in(with_default),
        ));
    }

    let target: AssignmentTarget<'a> = read_assignment_target(cx, node)?;

    Ok(AssignmentTargetMaybeDefault::from(target))
}

pub fn read_property_key<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<PropertyKey<'a>, ReadError> {
    if ty_of(node) == "PrivateIdentifier" {
        let private: PrivateIdentifier<'a> =
            PrivateIdentifier::new(cx.span(node), cx.name(node)?, cx.builder());

        return Ok(PropertyKey::PrivateIdentifier(cx.box_in(private)));
    }

    Ok(PropertyKey::from(read_expression(cx, node)?))
}

pub fn read_identifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let reference: IdentifierReference<'a> =
        IdentifierReference::new(cx.span(node), cx.name(node)?, cx.builder());

    Ok(Expression::Identifier(cx.box_in(reference)))
}

pub fn simple_assignment_target_from_expression<'a>(
    cx: &Cx<'a>,
    expression: Expression<'a>,
    node: &Value,
) -> Result<SimpleAssignmentTarget<'a>, ReadError> {
    match expression {
        | Expression::Identifier(ident) => {
            Ok(SimpleAssignmentTarget::AssignmentTargetIdentifier(ident))
        },
        | expr @ (Expression::ComputedMemberExpression(_)
        | Expression::StaticMemberExpression(_)
        | Expression::PrivateFieldExpression(_)) => {
            let member: MemberExpression<'a> =
                match MemberExpression::try_from(expr) {
                    | Ok(member) => member,
                    | Err(_) => return Err(cx.err(node)),
                };
            Ok(SimpleAssignmentTarget::from(member))
        },
        // TS-ESTree allows `x as T` / `x!` / `<T>x` as update/assignment
        // targets; oxc models them as `SimpleAssignmentTarget` variants.
        | Expression::TSAsExpression(inner) => {
            Ok(SimpleAssignmentTarget::TSAsExpression(inner))
        },
        | Expression::TSSatisfiesExpression(inner) => {
            Ok(SimpleAssignmentTarget::TSSatisfiesExpression(inner))
        },
        | Expression::TSNonNullExpression(inner) => {
            Ok(SimpleAssignmentTarget::TSNonNullExpression(inner))
        },
        | Expression::TSTypeAssertion(inner) => {
            Ok(SimpleAssignmentTarget::TSTypeAssertion(inner))
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_assignment_target<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<AssignmentTarget<'a>, ReadError> {
    match ty_of(node) {
        | "ArrayPattern" => array_assignment_target(cx, node),
        | "ObjectPattern" => object_assignment_target(cx, node),
        | _ => {
            let expression: Expression<'a> = read_expression(cx, node)?;
            expression_to_assignment_target(cx, expression, node)
        },
    }
}
