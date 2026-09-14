use crate::reader::json::Value;
use oxc::ast::ast::{
    BindingPattern, BlockStatement, BreakStatement, CatchClause,
    CatchParameter, ContinueStatement, DebuggerStatement, DoWhileStatement,
    EmptyStatement, Expression, ExpressionStatement, ForInStatement,
    ForOfStatement, ForStatement, ForStatementInit, ForStatementLeft,
    FunctionType, IfStatement, LabelIdentifier, LabeledStatement,
    ReturnStatement, Statement, SwitchCase, SwitchStatement, TSTypeAnnotation,
    ThrowStatement, TryStatement, VariableDeclaration, VariableDeclarationKind,
    VariableDeclarator, WhileStatement, WithStatement,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use super::declaration;
use super::engine::{Cx, Seg, nodes, ty_of};
use super::expression;
use super::pattern;
use crate::errors::read::ReadError;

pub fn read_statement<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Statement<'a>, ReadError> {
    let ty: &str = ty_of(node);

    match ty {
        | "VariableDeclaration" => Ok(Statement::VariableDeclaration(
            cx.box_in(variable_declaration(cx, node)?),
        )),
        | "FunctionDeclaration"
        | "TSDeclareFunction"
        | "TSEmptyBodyFunctionExpression" => {
            let function_type: FunctionType = match ty {
                | "TSDeclareFunction" => FunctionType::TSDeclareFunction,
                | "TSEmptyBodyFunctionExpression" => {
                    FunctionType::TSEmptyBodyFunctionExpression
                },
                | _ => FunctionType::FunctionDeclaration,
            };
            Ok(Statement::FunctionDeclaration(
                cx.box_in(declaration::read_function(cx, node, function_type)?),
            ))
        },
        | "ForStatement" => {
            Ok(Statement::ForStatement(cx.box_in(for_statement(cx, node)?)))
        },
        | "TryStatement" => {
            Ok(Statement::TryStatement(cx.box_in(try_statement(cx, node)?)))
        },
        | "ImportDeclaration" => Ok(Statement::ImportDeclaration(
            cx.box_in(declaration::read_import_declaration(cx, node)?),
        )),
        | "ClassDeclaration" => Ok(Statement::ClassDeclaration(
            cx.box_in(declaration::read_class_declaration(cx, node)?),
        )),
        | "TSTypeAliasDeclaration" => Ok(Statement::TSTypeAliasDeclaration(
            cx.box_in(declaration::read_ts_type_alias_declaration(cx, node)?),
        )),
        | "TSInterfaceDeclaration" => Ok(Statement::TSInterfaceDeclaration(
            cx.box_in(declaration::read_ts_interface_declaration(cx, node)?),
        )),
        | "TSEnumDeclaration" => Ok(Statement::TSEnumDeclaration(
            cx.box_in(declaration::read_ts_enum_declaration(cx, node)?),
        )),
        | "TSModuleDeclaration" => {
            declaration::read_ts_module_declaration(cx, node)
        },
        | "TSImportEqualsDeclaration" => {
            Ok(Statement::TSImportEqualsDeclaration(cx.box_in(
                declaration::read_ts_import_equals_declaration(cx, node)?,
            )))
        },
        | "TSExportAssignment" => Ok(Statement::TSExportAssignment(
            cx.box_in(declaration::read_ts_export_assignment(cx, node)?),
        )),
        | "ExportNamedDeclaration" => {
            declaration::read_export_named_declaration(cx, node)
        },
        | "ExportDefaultDeclaration" => {
            Ok(Statement::ExportDefaultDeclaration(cx.box_in(
                declaration::read_export_default_declaration(cx, node)?,
            )))
        },
        | "ExportAllDeclaration" => Ok(Statement::ExportAllDeclaration(
            cx.box_in(declaration::read_export_all_declaration(cx, node)?),
        )),
        | "BreakStatement" | "ContinueStatement" => {
            let label: Option<LabelIdentifier<'a>> = optional_label(cx, node)?;
            match ty {
                | "BreakStatement" => {
                    let stmt: BreakStatement<'a> =
                        BreakStatement::new(cx.span(node), label, cx.builder());
                    Ok(Statement::BreakStatement(cx.box_in(stmt)))
                },
                | _ => {
                    let stmt: ContinueStatement<'a> = ContinueStatement::new(
                        cx.span(node),
                        label,
                        cx.builder(),
                    );
                    Ok(Statement::ContinueStatement(cx.box_in(stmt)))
                },
            }
        },
        | "LabeledStatement" => {
            let label: LabelIdentifier<'a> =
                cx.req(node, "label", |cx: &Cx<'a>, label_node: &Value| {
                    Ok(LabelIdentifier::new(
                        cx.span(label_node),
                        cx.name(label_node)?,
                        cx.builder(),
                    ))
                })?;
            let body: Statement<'a> = cx.stmt(node, "body")?;
            let stmt: LabeledStatement<'a> =
                LabeledStatement::new(cx.span(node), label, body, cx.builder());
            Ok(Statement::LabeledStatement(cx.box_in(stmt)))
        },
        | _ => nodes! { cx, node, ty, Statement :
            "ExpressionStatement" => ExpressionStatement: ExpressionStatement::new [
                cx.span(node),
                cx.expr(node, "expression")?,
            ];
            "ReturnStatement" => ReturnStatement: ReturnStatement::new [
                cx.span(node),
                cx.opt_expr(node, "argument")?,
            ];
            "BlockStatement" => BlockStatement: BlockStatement::new [
                cx.span(node),
                cx.stmts(node, "body")?,
            ];
            "IfStatement" => IfStatement: IfStatement::new [
                cx.span(node),
                cx.expr(node, "test")?,
                cx.stmt(node, "consequent")?,
                cx.opt_stmt(node, "alternate")?,
            ];
            "ForInStatement" => ForInStatement: ForInStatement::new [
                cx.span(node),
                cx.req(node, "left", for_statement_left)?,
                cx.expr(node, "right")?,
                cx.stmt(node, "body")?,
            ];
            "ForOfStatement" => ForOfStatement: ForOfStatement::new [
                cx.span(node),
                cx.flag(node, "await"),
                cx.req(node, "left", for_statement_left)?,
                cx.expr(node, "right")?,
                cx.stmt(node, "body")?,
            ];
            "WhileStatement" => WhileStatement: WhileStatement::new [
                cx.span(node),
                cx.expr(node, "test")?,
                cx.stmt(node, "body")?,
            ];
            "DoWhileStatement" => DoWhileStatement: DoWhileStatement::new [
                cx.span(node),
                cx.stmt(node, "body")?,
                cx.expr(node, "test")?,
            ];
            "ThrowStatement" => ThrowStatement: ThrowStatement::new [
                cx.span(node),
                cx.expr(node, "argument")?,
            ];
            "SwitchStatement" => SwitchStatement: SwitchStatement::new [
                cx.span(node),
                cx.expr(node, "discriminant")?,
                switch_cases(cx, node)?,
            ];
            "WithStatement" => WithStatement: WithStatement::new [
                cx.span(node),
                cx.expr(node, "object")?,
                cx.stmt(node, "body")?,
            ];
            "DebuggerStatement" => DebuggerStatement: DebuggerStatement::new [
                cx.span(node),
            ];
            "EmptyStatement" => EmptyStatement: EmptyStatement::new [
                cx.span(node),
            ];
        },
    }
}

fn for_statement<'a>(
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

fn for_statement_init<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ForStatementInit<'a>, ReadError> {
    if ty_of(node) == "VariableDeclaration" {
        return Ok(ForStatementInit::VariableDeclaration(
            cx.box_in(variable_declaration(cx, node)?),
        ));
    }

    Ok(ForStatementInit::from(expression::read_expression(cx, node)?))
}

fn for_statement_left<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ForStatementLeft<'a>, ReadError> {
    if ty_of(node) == "VariableDeclaration" {
        return Ok(ForStatementLeft::VariableDeclaration(
            cx.box_in(variable_declaration(cx, node)?),
        ));
    }

    Ok(ForStatementLeft::from(pattern::read_assignment_target(cx, node)?))
}

fn try_statement<'a>(
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

pub fn block_statement<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BlockStatement<'a>, ReadError> {
    let block: BlockStatement<'a> = BlockStatement::new(
        cx.span(node),
        cx.stmts(node, "body")?,
        cx.builder(),
    );

    Ok(block)
}

fn catch_clause<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<CatchClause<'a>, ReadError> {
    let span: Span = cx.span(node);

    let param: Option<CatchParameter<'a>> =
        cx.opt(node, "param", |cx: &Cx<'a>, param_node: &Value| {
            let pattern: BindingPattern<'a> =
                pattern::read_binding_pattern(cx, param_node)?;

            let type_annotation = cx.opt_annotation(param_node)?;

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

fn switch_cases<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<oxc::allocator::Vec<'a, SwitchCase<'a>>, ReadError> {
    cx.list(node, "cases", |cx, item| {
        let test: Option<Expression<'a>> =
            cx.opt(item, "test", expression::read_expression)?;

        let case: SwitchCase<'a> = SwitchCase::new(
            cx.span(item),
            test,
            cx.stmts(item, "consequent")?,
            cx.builder(),
        );

        Ok(case)
    })
}

fn optional_label<'a>(
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

fn variable_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<VariableDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let kind: VariableDeclarationKind =
        match node.get("kind").and_then(Value::as_str) {
            | Some("var") => VariableDeclarationKind::Var,
            | Some("let") => VariableDeclarationKind::Let,
            | Some("const") => VariableDeclarationKind::Const,
            | Some(other) => {
                return Err(ReadError::from_message(format!(
                    "unsupported variable declaration kind `{other}` at {}",
                    cx.path_string()
                )));
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
        let id: BindingPattern<'a> =
            pattern::read_binding_pattern(cx, id_node)?;

        let type_annotation = cx.opt_annotation(id_node)?;

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
