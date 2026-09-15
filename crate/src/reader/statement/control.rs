use oxc::ast::ast::{
    BindingPattern, BlockStatement, CatchClause, CatchParameter, Expression,
    ForStatement, ForStatementInit, ForStatementLeft, LabelIdentifier,
    Statement, SwitchCase, TSTypeAnnotation, TryStatement, VariableDeclaration,
    VariableDeclarationKind, VariableDeclarator,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, Seg, ty_of};
use crate::reader::expression::expressions;
use crate::reader::json::Value;
use crate::reader::pattern::binding::read_binding_pattern;
use crate::reader::pattern::targets::read_assignment_target;
use crate::reader::statement::block_statement;

fn variable_declarator<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<VariableDeclarator<'a>, ReadError> {
    let span: Span = cx.span(node);

    let id_node: &Value = node.get("id").ok_or_else(|| cx.err(node))?;

    let (id, type_annotation): (
        BindingPattern<'a>,
        Option<oxc::allocator::Box<'a, TSTypeAnnotation<'a>>>,
    ) = cx.child(Seg::field("id"), |cx| {
        let id: BindingPattern<'a> = read_binding_pattern(cx, id_node)?;

        let type_annotation: Option<
            oxc::allocator::Box<'a, TSTypeAnnotation<'a>>,
        > = cx.annotation(id_node, "typeAnnotation")?;

        Ok((id, type_annotation))
    })?;

    let declarator: VariableDeclarator<'a> = VariableDeclarator::new(
        span,
        id,
        type_annotation,
        cx.opt_expr(node, "init")?,
        cx.flag(node, "definite"),
        cx.builder(),
    );

    Ok(declarator)
}

pub fn variable_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<VariableDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let kind: VariableDeclarationKind =
        match node.get("kind").and_then(Value::as_str) {
            | Some("var") => VariableDeclarationKind::Var,
            | Some("let") => VariableDeclarationKind::Let,
            | Some("const") => VariableDeclarationKind::Const,
            | Some("using") => VariableDeclarationKind::Using,
            | Some("await using") => VariableDeclarationKind::AwaitUsing,
            | Some(other) => {
                return Err(ReadError::ValueUnsupported {
                    kind: "variable declaration kind",
                    value: other.to_string(),
                    path: cx.path_string(),
                });
            },
            | None => return Err(cx.err(node)),
        };

    let declarations: oxc::allocator::Vec<'a, VariableDeclarator<'a>> =
        cx.list(node, "declarations", variable_declarator)?;

    let decl: VariableDeclaration<'a> = VariableDeclaration::new(
        span,
        kind,
        declarations,
        cx.flag(node, "declare"),
        cx.builder(),
    );

    Ok(decl)
}

fn catch_clause<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<CatchClause<'a>, ReadError> {
    let span: Span = cx.span(node);

    let param: Option<CatchParameter<'a>> =
        cx.opt(node, "param", |cx: &Cx<'a>, param_node: &Value| {
            let pattern: BindingPattern<'a> =
                read_binding_pattern(cx, param_node)?;

            let type_annotation =
                cx.annotation(param_node, "typeAnnotation")?;

            Ok(CatchParameter::new(
                cx.span(param_node),
                pattern,
                type_annotation,
                cx.builder(),
            ))
        })?;

    let body: BlockStatement<'a> = cx.req(node, "body", block_statement)?;

    Ok(CatchClause::new(span, param, cx.box_in(body), cx.builder()))
}

pub fn try_statement<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TryStatement<'a>, ReadError> {
    let span: Span = cx.span(node);

    let block: BlockStatement<'a> = cx.req(node, "block", block_statement)?;

    let handler: Option<oxc::allocator::Box<'a, CatchClause<'a>>> =
        cx.opt_box(node, "handler", catch_clause)?;

    let finalizer: Option<oxc::allocator::Box<'a, BlockStatement<'a>>> =
        cx.opt_box(node, "finalizer", block_statement)?;

    let stmt: TryStatement<'a> = TryStatement::new(
        span,
        cx.box_in(block),
        handler,
        finalizer,
        cx.builder(),
    );

    Ok(stmt)
}

pub fn optional_label<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Option<LabelIdentifier<'a>>, ReadError> {
    cx.opt(node, "label", |cx: &Cx<'a>, label_node: &Value| {
        Ok(LabelIdentifier::new(
            cx.span(label_node),
            cx.name(label_node)?,
            cx.builder(),
        ))
    })
}

pub fn switch_cases<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<oxc::allocator::Vec<'a, SwitchCase<'a>>, ReadError> {
    cx.list(node, "cases", |cx, item| {
        let test: Option<Expression<'a>> =
            cx.opt(item, "test", expressions::read_expression)?;

        let case: SwitchCase<'a> = SwitchCase::new(
            cx.span(item),
            test,
            cx.stmts(item, "consequent")?,
            cx.builder(),
        );

        Ok(case)
    })
}

pub fn for_statement_left<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ForStatementLeft<'a>, ReadError> {
    if ty_of(node) == "VariableDeclaration" {
        return Ok(ForStatementLeft::VariableDeclaration(
            cx.box_in(variable_declaration(cx, node)?),
        ));
    }

    Ok(ForStatementLeft::from(read_assignment_target(cx, node)?))
}

fn for_statement_init<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ForStatementInit<'a>, ReadError> {
    if ty_of(node) == "VariableDeclaration" {
        return Ok(ForStatementInit::VariableDeclaration(
            cx.box_in(variable_declaration(cx, node)?),
        ));
    }

    Ok(ForStatementInit::from(expressions::read_expression(cx, node)?))
}

pub fn for_statement<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ForStatement<'a>, ReadError> {
    let init: Option<ForStatementInit<'a>> =
        cx.opt(node, "init", for_statement_init)?;

    let body: Statement<'a> = cx.stmt(node, "body")?;

    let stmt: ForStatement<'a> = ForStatement::new(
        cx.span(node),
        init,
        cx.opt_expr(node, "test")?,
        cx.opt_expr(node, "update")?,
        body,
        cx.builder(),
    );

    Ok(stmt)
}
