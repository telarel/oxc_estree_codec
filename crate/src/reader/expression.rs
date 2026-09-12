use crate::reader::json::Value;
use oxc::allocator::{Box as ArenaBox, Vec as ArenaVec};
use oxc::ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, ArrowFunctionBody,
    ArrowFunctionExpression, AssignmentExpression, AwaitExpression,
    BinaryExpression, BinaryOperator, CallExpression, ChainElement,
    ChainExpression, ClassType, ComputedMemberExpression,
    ConditionalExpression, Elision, Expression, FormalParameterKind,
    FormalParameters, FunctionBody, FunctionType, IdentifierName,
    ImportExpression, ImportMeta, ImportPhase, LogicalExpression,
    LogicalOperator, MemberExpression, NewExpression, NewTarget,
    ObjectExpression, ObjectProperty, ObjectPropertyKind,
    ParenthesizedExpression, PrivateFieldExpression, PrivateIdentifier,
    PropertyKind, SequenceExpression, SimpleAssignmentTarget, SpreadElement,
    StaticMemberExpression, Super, TSAsExpression, TSNonNullExpression,
    TSSatisfiesExpression, TSThisParameter, TSTypeAssertion,
    TaggedTemplateExpression, TemplateElement, TemplateElementValue,
    TemplateLiteral, ThisExpression, UnaryExpression, UnaryOperator,
    UpdateExpression, YieldExpression,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use super::declaration;
use super::engine::{Cx, nodes, ty_of};
use super::jsx;
use super::literal;
use super::pattern;
use crate::errors::read::ReadError;

pub(crate) fn read_expression<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Expression<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "Literal" => literal::read_literal(cx, node),
        | "Identifier" => pattern::read_identifier(cx, node),
        | "MemberExpression" => member_expression(cx, node),
        | "TemplateLiteral" => read_template_literal(cx, node)
            .map(|template| Expression::TemplateLiteral(cx.box_in(template))),
        | "FunctionExpression" => Ok(Expression::FunctionExpression(
            cx.box_in(declaration::read_function(
                cx,
                node,
                FunctionType::FunctionExpression,
            )?),
        )),
        | "ClassExpression" => Ok(Expression::ClassExpression(cx.box_in(
            declaration::read_class(cx, node, ClassType::ClassExpression)?,
        ))),
        | "ArrowFunctionExpression" => Ok(Expression::ArrowFunctionExpression(
            cx.box_in(arrow_function(cx, node)?),
        )),
        | "ChainExpression" => chain_expression(cx, node),
        | "ImportExpression" => import_expression(cx, node),
        | "MetaProperty" => meta_property(cx, node),
        | "JSXElement" | "JSXFragment" => {
            jsx::read_jsx_expression(cx, node, ty)
        },
        | "UpdateExpression" => update_expression(cx, node),
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
                cx.req(node, "left", pattern::read_assignment_target)?,
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
        },
    }
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

fn read_template_literal<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TemplateLiteral<'a>, ReadError> {
    Ok(TemplateLiteral::new(
        cx.span(node),
        cx.list(node, "quasis", read_template_element)?,
        cx.exprs(node, "expressions")?,
        cx.builder(),
    ))
}

pub(crate) fn read_template_element<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TemplateElement<'a>, ReadError> {
    let mut span: Span = cx.span(node);

    let tail: bool = cx.flag(node, "tail");

    span.start += 1;

    span.end -= if tail { 1 } else { 2 };

    let value_node: &Value = node.get("value").ok_or_else(|| cx.err(node))?;

    let raw: &str =
        value_node.get("raw").and_then(Value::as_str).unwrap_or_default();

    let raw_str: oxc::str::Str<'a> =
        oxc::str::Str::from_str_in(raw, cx.builder());

    let cooked: Option<oxc::str::Str<'a>> = match value_node.get("cooked") {
        | Some(cooked_node) if cooked_node.is_str() => {
            let cooked_str: &str = cooked_node.as_str().unwrap_or_default();
            Some(oxc::str::Str::from_str_in(cooked_str, cx.builder()))
        },
        | _ => None,
    };

    let value: TemplateElementValue<'a> =
        TemplateElementValue { raw: raw_str, cooked };

    let element: TemplateElement<'a> =
        TemplateElement::new(span, value, tail, cx.builder());

    Ok(element)
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
            match read_expression(cx, argument_node)? {
                | Expression::Identifier(ident) => Ok(
                    SimpleAssignmentTarget::AssignmentTargetIdentifier(ident),
                ),
                | _ => Err(cx.err(argument_node)),
            }
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
    ) = declaration::read_formal_parameters(
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
                        super::engine::read_function_body(cx, body_node)?;
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
        None,
        cx.box_in(params),
        None,
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
                        return Err(ReadError::from_message(format!(
                            "unsupported import phase `{other}` at {}",
                            cx.path_string()
                        )));
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

    let meta_name: &str = node
        .get("meta")
        .and_then(|m| m.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let property_name: &str = node
        .get("property")
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("");

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

pub(crate) fn binary_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<BinaryOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: BinaryOperator = match op {
        | "==" => BinaryOperator::Equality,
        | "!=" => BinaryOperator::Inequality,
        | "===" => BinaryOperator::StrictEquality,
        | "!==" => BinaryOperator::StrictInequality,
        | "<" => BinaryOperator::LessThan,
        | "<=" => BinaryOperator::LessEqualThan,
        | ">" => BinaryOperator::GreaterThan,
        | ">=" => BinaryOperator::GreaterEqualThan,
        | "+" => BinaryOperator::Addition,
        | "-" => BinaryOperator::Subtraction,
        | "*" => BinaryOperator::Multiplication,
        | "/" => BinaryOperator::Division,
        | "%" => BinaryOperator::Remainder,
        | "**" => BinaryOperator::Exponential,
        | "<<" => BinaryOperator::ShiftLeft,
        | ">>" => BinaryOperator::ShiftRight,
        | ">>>" => BinaryOperator::ShiftRightZeroFill,
        | "|" => BinaryOperator::BitwiseOR,
        | "^" => BinaryOperator::BitwiseXOR,
        | "&" => BinaryOperator::BitwiseAnd,
        | "in" => BinaryOperator::In,
        | "instanceof" => BinaryOperator::Instanceof,
        | _ => {
            return Err(ReadError::from_message(format!(
                "unsupported binary operator `{op}` at {}",
                cx.path_string()
            )));
        },
    };

    Ok(operator)
}

pub(crate) fn logical_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<LogicalOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: LogicalOperator = match op {
        | "||" => LogicalOperator::Or,
        | "&&" => LogicalOperator::And,
        | "??" => LogicalOperator::Coalesce,
        | _ => {
            return Err(ReadError::from_message(format!(
                "unsupported logical operator `{op}` at {}",
                cx.path_string()
            )));
        },
    };

    Ok(operator)
}

pub(crate) fn unary_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<UnaryOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: UnaryOperator = match op {
        | "+" => UnaryOperator::UnaryPlus,
        | "-" => UnaryOperator::UnaryNegation,
        | "!" => UnaryOperator::LogicalNot,
        | "~" => UnaryOperator::BitwiseNot,
        | "typeof" => UnaryOperator::Typeof,
        | "void" => UnaryOperator::Void,
        | "delete" => UnaryOperator::Delete,
        | _ => {
            return Err(ReadError::from_message(format!(
                "unsupported unary operator `{op}` at {}",
                cx.path_string()
            )));
        },
    };

    Ok(operator)
}

pub(crate) fn update_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<oxc::syntax::operator::UpdateOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: oxc::syntax::operator::UpdateOperator = match op {
        | "++" => oxc::syntax::operator::UpdateOperator::Increment,
        | "--" => oxc::syntax::operator::UpdateOperator::Decrement,
        | _ => {
            return Err(ReadError::from_message(format!(
                "unsupported update operator `{op}` at {}",
                cx.path_string()
            )));
        },
    };

    Ok(operator)
}

pub(crate) fn assignment_operator(
    cx: &Cx<'_>,
    node: &Value,
) -> Result<oxc::syntax::operator::AssignmentOperator, ReadError> {
    let op: &str = node.get("operator").and_then(Value::as_str).unwrap_or("");

    let operator: oxc::syntax::operator::AssignmentOperator = match op {
        | "=" => oxc::syntax::operator::AssignmentOperator::Assign,
        | "+=" => oxc::syntax::operator::AssignmentOperator::Addition,
        | "-=" => oxc::syntax::operator::AssignmentOperator::Subtraction,
        | "*=" => oxc::syntax::operator::AssignmentOperator::Multiplication,
        | "/=" => oxc::syntax::operator::AssignmentOperator::Division,
        | "%=" => oxc::syntax::operator::AssignmentOperator::Remainder,
        | "**=" => oxc::syntax::operator::AssignmentOperator::Exponential,
        | "<<=" => oxc::syntax::operator::AssignmentOperator::ShiftLeft,
        | ">>=" => oxc::syntax::operator::AssignmentOperator::ShiftRight,
        | ">>>=" => {
            oxc::syntax::operator::AssignmentOperator::ShiftRightZeroFill
        },
        | "|=" => oxc::syntax::operator::AssignmentOperator::BitwiseOR,
        | "^=" => oxc::syntax::operator::AssignmentOperator::BitwiseXOR,
        | "&=" => oxc::syntax::operator::AssignmentOperator::BitwiseAnd,
        | "||=" => oxc::syntax::operator::AssignmentOperator::LogicalOr,
        | "&&=" => oxc::syntax::operator::AssignmentOperator::LogicalAnd,
        | "??=" => oxc::syntax::operator::AssignmentOperator::LogicalNullish,
        | _ => {
            return Err(ReadError::from_message(format!(
                "unsupported assignment operator `{op}` at {}",
                cx.path_string()
            )));
        },
    };

    Ok(operator)
}
