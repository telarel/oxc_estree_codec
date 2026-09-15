use oxc::allocator::{Box as ArenaBox, Vec as ArenaVec};
use oxc::ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, ArrowFunctionBody,
    ArrowFunctionExpression, AssignmentExpression, AwaitExpression,
    BinaryExpression, CallExpression, ChainElement, ChainExpression, ClassType,
    ComputedMemberExpression, ConditionalExpression, Elision, Expression,
    FormalParameterKind, FormalParameters, FunctionBody, FunctionType,
    IdentifierName, ImportExpression, ImportMeta, ImportPhase,
    LogicalExpression, MemberExpression, NewExpression, NewTarget,
    ObjectExpression, ObjectProperty, ObjectPropertyKind,
    ParenthesizedExpression, PrivateFieldExpression, PrivateIdentifier,
    PrivateInExpression, PropertyKind, SequenceExpression,
    SimpleAssignmentTarget, SpreadElement, StaticMemberExpression, Super,
    TSAsExpression, TSInstantiationExpression, TSNonNullExpression,
    TSSatisfiesExpression, TSThisParameter, TSTypeAssertion,
    TaggedTemplateExpression, ThisExpression, UnaryExpression,
    UpdateExpression, YieldExpression,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::declaration::class;
use crate::reader::declaration::function;
use crate::reader::engine::context::{Cx, nodes, ty_of};
use crate::reader::engine::program::read_function_body;
use crate::reader::expression::operators::{
    assignment_operator, binary_operator, logical_operator, unary_operator,
    update_operator,
};
use crate::reader::expression::templates::read_template_literal;
use crate::reader::json::Value;
use crate::reader::jsx::element;
use crate::reader::literal;
use crate::reader::pattern;

fn is_private_in_expression(node: &Value) -> bool {
    node.get("operator").and_then(Value::as_str) == Some("in")
        && node
            .get("left")
            .and_then(|left| left.get("type"))
            .and_then(Value::as_str)
            == Some("PrivateIdentifier")
}

fn private_in_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let left_node: &Value = node.get("left").ok_or_else(|| cx.err(node))?;

    let left: PrivateIdentifier<'a> = PrivateIdentifier::new(
        cx.span(left_node),
        cx.name(left_node)?,
        cx.builder(),
    );

    let right: Expression<'a> = cx.expr(node, "right")?;

    Ok(Expression::PrivateInExpression(cx.box_in(PrivateInExpression::new(
        span,
        left,
        right,
        cx.builder(),
    ))))
}

fn member_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let object: Expression<'a> = cx.expr(node, "object")?;

    let optional: bool = cx.flag(node, "optional");

    if cx.flag(node, "computed") {
        let property: Expression<'a> = cx.expr(node, "property")?;
        let computed: ComputedMemberExpression<'a> =
            ComputedMemberExpression::new(
                span,
                object,
                property,
                optional,
                cx.builder(),
            );
        return Ok(Expression::from(
            MemberExpression::ComputedMemberExpression(cx.box_in(computed)),
        ));
    }

    let property_node: &Value =
        node.get("property").ok_or_else(|| cx.err(node))?;

    if ty_of(property_node) == "PrivateIdentifier" {
        let field: PrivateIdentifier<'a> = PrivateIdentifier::new(
            cx.span(property_node),
            cx.name(property_node)?,
            cx.builder(),
        );

        let private: PrivateFieldExpression<'a> = PrivateFieldExpression::new(
            span,
            object,
            field,
            optional,
            cx.builder(),
        );

        return Ok(Expression::from(MemberExpression::PrivateFieldExpression(
            cx.box_in(private),
        )));
    }

    let identifier_name: IdentifierName<'a> = IdentifierName::new(
        cx.span(property_node),
        cx.name(property_node)?,
        cx.builder(),
    );

    let static_member: StaticMemberExpression<'a> = StaticMemberExpression::new(
        span,
        object,
        identifier_name,
        optional,
        cx.builder(),
    );

    Ok(Expression::from(MemberExpression::StaticMemberExpression(
        cx.box_in(static_member),
    )))
}

fn array_elements<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ArenaVec<'a, ArrayExpressionElement<'a>>, ReadError> {
    cx.list(node, "elements", |cx, item| {
        if item.is_null() {
            let elision: Elision = Elision::new(cx.span(item), cx.builder());
            return Ok(ArrayExpressionElement::Elision(cx.box_in(elision)));
        }

        if ty_of(item) == "SpreadElement" {
            let spread: SpreadElement<'a> = SpreadElement::new(
                cx.span(item),
                cx.expr(item, "argument")?,
                cx.builder(),
            );

            return Ok(ArrayExpressionElement::SpreadElement(
                cx.box_in(spread),
            ));
        }

        Ok(ArrayExpressionElement::from(read_expression(cx, item)?))
    })
}

fn object_property<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ObjectProperty<'a>, ReadError> {
    let kind: PropertyKind = match node.get("kind").and_then(Value::as_str) {
        | Some("init") => PropertyKind::Init,
        | Some("get") => PropertyKind::Get,
        | Some("set") => PropertyKind::Set,
        | _ => return Err(cx.err(node)),
    };

    Ok(ObjectProperty::new(
        cx.span(node),
        kind,
        cx.property_key(node, "key")?,
        cx.expr(node, "value")?,
        cx.flag(node, "method"),
        cx.flag(node, "shorthand"),
        cx.flag(node, "computed"),
        cx.builder(),
    ))
}

fn object_properties<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ArenaVec<'a, ObjectPropertyKind<'a>>, ReadError> {
    cx.list(node, "properties", |cx, item| {
        if ty_of(item) == "SpreadElement" {
            let spread: SpreadElement<'a> = SpreadElement::new(
                cx.span(item),
                cx.expr(item, "argument")?,
                cx.builder(),
            );

            return Ok(ObjectPropertyKind::SpreadProperty(cx.box_in(spread)));
        }

        Ok(ObjectPropertyKind::ObjectProperty(
            cx.box_in(object_property(cx, item)?),
        ))
    })
}

fn arguments<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ArenaVec<'a, Argument<'a>>, ReadError> {
    cx.list(node, "arguments", |cx, item| {
        if ty_of(item) == "SpreadElement" {
            let spread: SpreadElement<'a> = SpreadElement::new(
                cx.span(item),
                cx.expr(item, "argument")?,
                cx.builder(),
            );

            return Ok(Argument::SpreadElement(cx.box_in(spread)));
        }

        Ok(Argument::from(read_expression(cx, item)?))
    })
}

fn update_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let operator: oxc::syntax::operator::UpdateOperator =
        update_operator(cx, node)?;

    let prefix: bool = cx.flag_or(node, "prefix", true);

    let target: SimpleAssignmentTarget<'a> =
        cx.req(node, "argument", |cx: &Cx<'a>, argument_node: &Value| {
            let expression: Expression<'a> =
                read_expression(cx, argument_node)?;

            pattern::targets::simple_assignment_target_from_expression(
                cx,
                expression,
                argument_node,
            )
        })?;

    let update: UpdateExpression<'a> =
        UpdateExpression::new(span, operator, prefix, target, cx.builder());

    Ok(Expression::UpdateExpression(cx.box_in(update)))
}

fn arrow_function<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ArrowFunctionExpression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let is_async: bool = cx.flag(node, "async");

    let (params, this_param): (
        FormalParameters<'a>,
        Option<ArenaBox<'a, TSThisParameter<'a>>>,
    ) = function::read_formal_parameters(
        cx,
        node,
        span,
        FormalParameterKind::ArrowFormalParameters,
    )?;

    if this_param.is_some() {
        return Err(cx.err(node));
    }

    let body: ArrowFunctionBody<'a> =
        cx.req(node, "body", |cx: &Cx<'a>, body_node: &Value| {
            match ty_of(body_node) {
                | "BlockStatement" => {
                    let function_body: FunctionBody<'a> =
                        read_function_body(cx, body_node)?;
                    Ok(ArrowFunctionBody::FunctionBody(
                        cx.box_in(function_body),
                    ))
                },
                | _ => {
                    Ok(ArrowFunctionBody::from(read_expression(cx, body_node)?))
                },
            }
        })?;

    let arrow: ArrowFunctionExpression<'a> = ArrowFunctionExpression::new(
        span,
        is_async,
        cx.opt_type_parameters(node)?,
        cx.box_in(params),
        cx.annotation(node, "returnType")?,
        body,
        cx.builder(),
    );

    Ok(arrow)
}

fn chain_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let element: ChainElement<'a> =
        cx.req(node, "expression", |cx: &Cx<'a>, inner_node: &Value| {
            match read_expression(cx, inner_node)? {
                | Expression::CallExpression(call) => {
                    Ok(ChainElement::CallExpression(call))
                },
                | Expression::TSNonNullExpression(non_null) => {
                    Ok(ChainElement::TSNonNullExpression(non_null))
                },
                | expr => match MemberExpression::try_from(expr) {
                    | Ok(member) => Ok(ChainElement::from(member)),
                    | Err(_) => Err(cx.err(inner_node)),
                },
            }
        })?;

    let chain: ChainExpression<'a> =
        ChainExpression::new(span, element, cx.builder());

    Ok(Expression::ChainExpression(cx.box_in(chain)))
}

fn import_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let source: Expression<'a> = cx.expr(node, "source")?;

    let options: Option<Expression<'a>> =
        cx.opt(node, "options", read_expression)?;

    let phase: Option<ImportPhase> =
        match node.get("phase").filter(|v| !v.is_null()) {
            | Some(phase_node) => {
                let phase_str: &str =
                    phase_node.as_str().ok_or_else(|| cx.err(node))?;

                let phase: ImportPhase = match phase_str {
                    | "source" => ImportPhase::Source,
                    | "defer" => ImportPhase::Defer,
                    | other => {
                        return Err(ReadError::ImportPhaseUnsupported {
                            phase: other.to_string(),
                            path: cx.path_string(),
                        });
                    },
                };

                Some(phase)
            },
            | None => None,
        };

    let import: ImportExpression<'a> =
        ImportExpression::new(span, source, options, phase, cx.builder());

    Ok(Expression::ImportExpression(cx.box_in(import)))
}

fn meta_property<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let span: Span = cx.span(node);

    let meta_node: &Value =
        node.get("meta").ok_or_else(|| cx.missing(node, "meta"))?;

    let property_node: &Value =
        node.get("property").ok_or_else(|| cx.missing(node, "property"))?;

    let meta_name: &str =
        meta_node.get("name").and_then(Value::as_str).unwrap_or_default();

    let property_name: &str =
        property_node.get("name").and_then(Value::as_str).unwrap_or_default();

    if meta_name == "import" && property_name == "meta" {
        let meta: ImportMeta = ImportMeta::new(span, cx.builder());
        Ok(Expression::ImportMeta(cx.box_in(meta)))
    } else if meta_name == "new" && property_name == "target" {
        let target: NewTarget = NewTarget::new(span, cx.builder());
        Ok(Expression::NewTarget(cx.box_in(target)))
    } else {
        Err(cx.err(node))
    }
}

pub fn read_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "Literal" => literal::read_literal(cx, node),
        | "Identifier" => pattern::targets::read_identifier(cx, node),
        | "MemberExpression" => member_expression(cx, node),
        | "TemplateLiteral" => read_template_literal(cx, node)
            .map(|template| Expression::TemplateLiteral(cx.box_in(template))),
        | "FunctionExpression" => Ok(Expression::FunctionExpression(
            cx.box_in(function::read_function(
                cx,
                node,
                FunctionType::FunctionExpression,
            )?),
        )),
        | "ClassExpression" => Ok(Expression::ClassExpression(
            cx.box_in(class::read_class(cx, node, ClassType::ClassExpression)?),
        )),
        | "ArrowFunctionExpression" => Ok(Expression::ArrowFunctionExpression(
            cx.box_in(arrow_function(cx, node)?),
        )),
        | "ChainExpression" => chain_expression(cx, node),
        | "ImportExpression" => import_expression(cx, node),
        | "MetaProperty" => meta_property(cx, node),
        | "JSXElement" | "JSXFragment" => {
            element::read_jsx_expression(cx, node, ty)
        },
        | "UpdateExpression" => update_expression(cx, node),
        | "BinaryExpression" if is_private_in_expression(node) => {
            private_in_expression(cx, node)
        },
        | _ => nodes! { cx, node, ty, Expression :
            "ThisExpression" => ThisExpression: ThisExpression::new [
                cx.span(node),
            ];
            "Super" => Super: Super::new [
                cx.span(node),
            ];
            "ArrayExpression" => ArrayExpression: ArrayExpression::new [
                cx.span(node),
                array_elements(cx, node)?,
            ];
            "ObjectExpression" => ObjectExpression: ObjectExpression::new [
                cx.span(node),
                object_properties(cx, node)?,
            ];
            "TaggedTemplateExpression" => TaggedTemplateExpression: TaggedTemplateExpression::new [
                cx.span(node),
                cx.expr(node, "tag")?,
                cx.opt_type_arguments(node)?,
                cx.req(node, "quasi", read_template_literal)?,
            ];
            "CallExpression" => CallExpression: CallExpression::new [
                cx.span(node),
                cx.expr(node, "callee")?,
                cx.opt_type_arguments(node)?,
                arguments(cx, node)?,
                cx.flag(node, "optional"),
            ];
            "NewExpression" => NewExpression: NewExpression::new [
                cx.span(node),
                cx.expr(node, "callee")?,
                cx.opt_type_arguments(node)?,
                arguments(cx, node)?,
            ];
            "BinaryExpression" => BinaryExpression: BinaryExpression::new [
                cx.span(node),
                cx.expr(node, "left")?,
                binary_operator(cx, node)?,
                cx.expr(node, "right")?,
            ];
            "LogicalExpression" => LogicalExpression: LogicalExpression::new [
                cx.span(node),
                cx.expr(node, "left")?,
                logical_operator(cx, node)?,
                cx.expr(node, "right")?,
            ];
            "UnaryExpression" => UnaryExpression: UnaryExpression::new [
                cx.span(node),
                unary_operator(cx, node)?,
                cx.expr(node, "argument")?,
            ];
            "AssignmentExpression" => AssignmentExpression: AssignmentExpression::new [
                cx.span(node),
                assignment_operator(cx, node)?,
                cx.req(node, "left", pattern::targets::read_assignment_target)?,
                cx.expr(node, "right")?,
            ];
            "ConditionalExpression" => ConditionalExpression: ConditionalExpression::new [
                cx.span(node),
                cx.expr(node, "test")?,
                cx.expr(node, "consequent")?,
                cx.expr(node, "alternate")?,
            ];
            "SequenceExpression" => SequenceExpression: SequenceExpression::new [
                cx.span(node),
                cx.exprs(node, "expressions")?,
            ];
            "AwaitExpression" => AwaitExpression: AwaitExpression::new [
                cx.span(node),
                cx.expr(node, "argument")?,
            ];
            "YieldExpression" => YieldExpression: YieldExpression::new [
                cx.span(node),
                cx.flag(node, "delegate"),
                cx.opt_expr(node, "argument")?,
            ];
            "ParenthesizedExpression" => ParenthesizedExpression: ParenthesizedExpression::new [
                cx.span(node),
                cx.expr(node, "expression")?,
            ];
            "TSAsExpression" => TSAsExpression: TSAsExpression::new [
                cx.span(node),
                cx.expr(node, "expression")?,
                cx.ts(node, "typeAnnotation")?,
            ];
            "TSSatisfiesExpression" => TSSatisfiesExpression: TSSatisfiesExpression::new [
                cx.span(node),
                cx.expr(node, "expression")?,
                cx.ts(node, "typeAnnotation")?,
            ];
            "TSNonNullExpression" => TSNonNullExpression: TSNonNullExpression::new [
                cx.span(node),
                cx.expr(node, "expression")?,
            ];
            "TSTypeAssertion" => TSTypeAssertion: TSTypeAssertion::new [
                cx.span(node),
                cx.ts(node, "typeAnnotation")?,
                cx.expr(node, "expression")?,
            ];
            "TSInstantiationExpression" => TSInstantiationExpression: TSInstantiationExpression::new [
                cx.span(node),
                cx.expr(node, "expression")?,
                cx.box_in(cx.req(
                    node,
                    "typeArguments",
                    crate::reader::ts_types::types::read_ts_type_parameter_instantiation,
                )?),
            ];
        },
    }
}
