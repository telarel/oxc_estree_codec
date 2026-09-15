pub mod control;

use oxc::ast::ast::{
    BlockStatement, BreakStatement, ContinueStatement, DebuggerStatement,
    DoWhileStatement, EmptyStatement, ExpressionStatement, ForInStatement,
    ForOfStatement, FunctionType, IfStatement, LabelIdentifier,
    LabeledStatement, ReturnStatement, Statement, SwitchStatement,
    ThrowStatement, WhileStatement, WithStatement,
};

use crate::errors::read::ReadError;
use crate::reader::declaration::class;
use crate::reader::declaration::enumeration;
use crate::reader::declaration::function;
use crate::reader::declaration::import_export;
use crate::reader::declaration::interface;
use crate::reader::declaration::module;
use crate::reader::engine::context::{Cx, nodes, ty_of};
use crate::reader::json::Value;
use crate::reader::statement::control::{
    for_statement, for_statement_left, optional_label, switch_cases,
    try_statement, variable_declaration,
};

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
                cx.box_in(function::read_function(cx, node, function_type)?),
            ))
        },
        | "ForStatement" => {
            Ok(Statement::ForStatement(cx.box_in(for_statement(cx, node)?)))
        },
        | "TryStatement" => {
            Ok(Statement::TryStatement(cx.box_in(try_statement(cx, node)?)))
        },
        | "ImportDeclaration" => Ok(Statement::ImportDeclaration(
            cx.box_in(import_export::read_import_declaration(cx, node)?),
        )),
        | "ClassDeclaration" => Ok(Statement::ClassDeclaration(
            cx.box_in(class::read_class_declaration(cx, node)?),
        )),
        | "TSTypeAliasDeclaration" => Ok(Statement::TSTypeAliasDeclaration(
            cx.box_in(interface::read_ts_type_alias_declaration(cx, node)?),
        )),
        | "TSInterfaceDeclaration" => Ok(Statement::TSInterfaceDeclaration(
            cx.box_in(interface::read_ts_interface_declaration(cx, node)?),
        )),
        | "TSEnumDeclaration" => Ok(Statement::TSEnumDeclaration(
            cx.box_in(enumeration::read_ts_enum_declaration(cx, node)?),
        )),
        | "TSModuleDeclaration" => module::read_ts_module_declaration(cx, node),
        | "TSImportEqualsDeclaration" => {
            Ok(Statement::TSImportEqualsDeclaration(
                cx.box_in(module::read_ts_import_equals_declaration(cx, node)?),
            ))
        },
        | "TSExportAssignment" => Ok(Statement::TSExportAssignment(
            cx.box_in(import_export::read_ts_export_assignment(cx, node)?),
        )),
        | "TSNamespaceExportDeclaration" => {
            Ok(Statement::TSNamespaceExportDeclaration(cx.box_in(
                import_export::read_ts_namespace_export_declaration(cx, node)?,
            )))
        },
        | "ExportNamedDeclaration" => {
            import_export::read_export_named_declaration(cx, node)
        },
        | "ExportDefaultDeclaration" => {
            Ok(Statement::ExportDefaultDeclaration(cx.box_in(
                import_export::read_export_default_declaration(cx, node)?,
            )))
        },
        | "ExportAllDeclaration" => Ok(Statement::ExportAllDeclaration(
            cx.box_in(import_export::read_export_all_declaration(cx, node)?),
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
