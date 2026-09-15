use oxc::allocator::{CloneIn, GetAllocator};
use oxc::ast::ast::{
    BindingIdentifier, Declaration, ExportAllDeclaration, ExportDeclaration,
    ExportDefaultDeclaration, ExportDefaultDeclarationKind,
    ExportFromDeclaration, ExportNamedDeclaration, ExportSpecifier, Expression,
    FunctionType, IdentifierName, ImportAttribute, ImportAttributeKey,
    ImportDeclaration, ImportDeclarationSpecifier, ImportDefaultSpecifier,
    ImportNamespaceSpecifier, ImportOrExportKind, ImportPhase, ImportSpecifier,
    ModuleExportName, Statement, StringLiteral, TSExportAssignment,
    TSNamespaceExportDeclaration, WithClause, WithClauseKeyword,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::declaration::class::read_class_declaration;
use crate::reader::declaration::function::{
    read_binding_identifier_at, read_function,
};
use crate::reader::declaration::interface::read_ts_interface_declaration;
use crate::reader::engine::context::{Cx, Seg, ty_of};
use crate::reader::expression::expressions;
use crate::reader::json::Value;
use crate::reader::literal;
use crate::reader::statement;

fn local_binding_identifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingIdentifier<'a>, ReadError> {
    cx.req(node, "local", read_binding_identifier_at)
}

fn read_module_export_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ModuleExportName<'a>, ReadError> {
    match ty_of(node) {
        | "Identifier" => Ok(ModuleExportName::IdentifierName(
            IdentifierName::new(cx.span(node), cx.name(node)?, cx.builder()),
        )),
        | "Literal" => Ok(ModuleExportName::StringLiteral(
            literal::read_string_literal(cx, node)?,
        )),
        | _ => Err(cx.err(node)),
    }
}

fn import_attribute<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ImportAttribute<'a>, ReadError> {
    let span: Span = cx.span(node);

    let key_node: &Value = node.get("key").ok_or_else(|| cx.err(node))?;

    let key: ImportAttributeKey<'a> =
        cx.child(Seg::field("key"), |cx| match ty_of(key_node) {
            | "Identifier" => {
                Ok(ImportAttributeKey::Identifier(IdentifierName::new(
                    cx.span(key_node),
                    cx.name(key_node)?,
                    cx.builder(),
                )))
            },
            | "Literal" => Ok(ImportAttributeKey::StringLiteral(
                literal::read_string_literal(cx, key_node)?,
            )),
            | _ => Err(cx.err(key_node)),
        })?;

    Ok(ImportAttribute::new(
        span,
        key,
        cx.string_literal(node, "value")?,
        cx.builder(),
    ))
}

fn read_with_clause<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Option<oxc::allocator::Box<'a, WithClause<'a>>>, ReadError> {
    let entries: Option<oxc::allocator::Vec<'a, ImportAttribute<'a>>> =
        cx.list_opt(node, "attributes", import_attribute)?;

    Ok(entries.map(|entries| {
        cx.box_in(WithClause::new(
            cx.span(node),
            WithClauseKeyword::With,
            entries,
            cx.builder(),
        ))
    }))
}

pub fn read_import_or_export_kind(
    cx: &Cx<'_>,
    kind: Option<&str>,
) -> Result<ImportOrExportKind, ReadError> {
    match kind {
        | Some("type") => Ok(ImportOrExportKind::Type),
        | Some("value") | None => Ok(ImportOrExportKind::Value),
        | Some(other) => Err(ReadError::ValueUnsupported {
            kind: "import/export kind",
            value: other.to_string(),
            path: cx.path_string(),
        }),
    }
}

fn import_specifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ImportDeclarationSpecifier<'a>, ReadError> {
    match ty_of(node) {
        | "ImportDefaultSpecifier" => {
            let spec: ImportDefaultSpecifier<'a> = ImportDefaultSpecifier::new(
                cx.span(node),
                local_binding_identifier(cx, node)?,
                cx.builder(),
            );

            Ok(ImportDeclarationSpecifier::ImportDefaultSpecifier(
                cx.box_in(spec),
            ))
        },
        | "ImportNamespaceSpecifier" => {
            let spec: ImportNamespaceSpecifier<'a> =
                ImportNamespaceSpecifier::new(
                    cx.span(node),
                    local_binding_identifier(cx, node)?,
                    cx.builder(),
                );

            Ok(ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                cx.box_in(spec),
            ))
        },
        | "ImportSpecifier" => {
            let spec: ImportSpecifier<'a> = ImportSpecifier::new(
                cx.span(node),
                cx.req(node, "imported", read_module_export_name)?,
                local_binding_identifier(cx, node)?,
                read_import_or_export_kind(
                    cx,
                    node.get("importKind").and_then(Value::as_str),
                )?,
                cx.builder(),
            );

            Ok(ImportDeclarationSpecifier::ImportSpecifier(cx.box_in(spec)))
        },
        | _ => Err(cx.err(node)),
    }
}

fn export_specifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ExportSpecifier<'a>, ReadError> {
    let span: Span = cx.span(node);

    let local: ModuleExportName<'a> =
        cx.req(node, "local", read_module_export_name)?;

    let exported: ModuleExportName<'a> = match node.get("exported") {
        | Some(exported_node) if !exported_node.is_null() => cx
            .child(Seg::field("exported"), |cx| {
                read_module_export_name(cx, exported_node)
            })?,
        | _ => local.clone_in(cx.builder().allocator()),
    };

    Ok(ExportSpecifier::new(
        span,
        local,
        exported,
        read_import_or_export_kind(
            cx,
            node.get("exportKind").and_then(Value::as_str),
        )?,
        cx.builder(),
    ))
}

pub fn read_import_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ImportDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let phase: Option<ImportPhase> =
        match node.get("phase").and_then(Value::as_str) {
            | Some("source") => Some(ImportPhase::Source),
            | Some("defer") => Some(ImportPhase::Defer),
            | _ => None,
        };

    let decl: ImportDeclaration<'a> = ImportDeclaration::new(
        span,
        cx.list_opt(node, "specifiers", import_specifier)?,
        cx.string_literal(node, "source")?,
        phase,
        read_with_clause(cx, node)?,
        read_import_or_export_kind(
            cx,
            node.get("importKind").and_then(Value::as_str),
        )?,
        cx.builder(),
    );

    Ok(decl)
}

pub fn read_export_named_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Statement<'a>, ReadError> {
    let span: Span = cx.span(node);

    let export_kind: ImportOrExportKind = read_import_or_export_kind(
        cx,
        node.get("exportKind").and_then(Value::as_str),
    )?;

    if let Some(declaration_node) =
        node.get("declaration").filter(|d| !d.is_null())
    {
        let declaration: Declaration<'a> =
            cx.child(Seg::field("declaration"), |cx| {
                let inner: Statement<'a> =
                    statement::read_statement(cx, declaration_node)?;
                Declaration::try_from(inner)
                    .map_err(|()| cx.err(declaration_node))
            })?;

        let decl: ExportDeclaration<'a> =
            ExportDeclaration::new(span, declaration, cx.builder());

        return Ok(Statement::ExportDeclaration(cx.box_in(decl)));
    }

    let specifiers: oxc::allocator::Vec<'a, ExportSpecifier<'a>> =
        cx.list(node, "specifiers", export_specifier)?;

    if let Some(source) =
        cx.opt(node, "source", literal::read_string_literal)?
    {
        let decl: ExportFromDeclaration<'a> = ExportFromDeclaration::new(
            span,
            specifiers,
            source,
            export_kind,
            read_with_clause(cx, node)?,
            cx.builder(),
        );

        return Ok(Statement::ExportFromDeclaration(cx.box_in(decl)));
    }

    let decl: ExportNamedDeclaration<'a> = ExportNamedDeclaration::new(
        span,
        specifiers,
        export_kind,
        cx.builder(),
    );

    Ok(Statement::ExportNamedDeclaration(cx.box_in(decl)))
}

pub fn read_export_default_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ExportDefaultDeclaration<'a>, ReadError> {
    let declaration: ExportDefaultDeclarationKind<'a> = cx.req(
        node,
        "declaration",
        |cx: &Cx<'a>, declaration_node: &Value| match ty_of(declaration_node) {
            | "FunctionDeclaration" => {
                Ok(ExportDefaultDeclarationKind::FunctionDeclaration(
                    cx.box_in(read_function(
                        cx,
                        declaration_node,
                        FunctionType::FunctionDeclaration,
                    )?),
                ))
            },
            | "TSDeclareFunction" => {
                Ok(ExportDefaultDeclarationKind::FunctionDeclaration(
                    cx.box_in(read_function(
                        cx,
                        declaration_node,
                        FunctionType::TSDeclareFunction,
                    )?),
                ))
            },
            | "ClassDeclaration" => {
                Ok(ExportDefaultDeclarationKind::ClassDeclaration(
                    cx.box_in(read_class_declaration(cx, declaration_node)?),
                ))
            },
            | "TSInterfaceDeclaration" => {
                Ok(ExportDefaultDeclarationKind::TSInterfaceDeclaration(
                    cx.box_in(read_ts_interface_declaration(
                        cx,
                        declaration_node,
                    )?),
                ))
            },
            | _ => Ok(ExportDefaultDeclarationKind::from(
                expressions::read_expression(cx, declaration_node)?,
            )),
        },
    )?;

    Ok(ExportDefaultDeclaration::new(cx.span(node), declaration, cx.builder()))
}

pub fn read_export_all_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<ExportAllDeclaration<'a>, ReadError> {
    let source: StringLiteral<'a> = cx.string_literal(node, "source")?;

    Ok(ExportAllDeclaration::new(
        cx.span(node),
        cx.opt(node, "exported", read_module_export_name)?,
        source,
        read_with_clause(cx, node)?,
        read_import_or_export_kind(
            cx,
            node.get("exportKind").and_then(Value::as_str),
        )?,
        cx.builder(),
    ))
}

pub fn read_ts_export_assignment<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSExportAssignment<'a>, ReadError> {
    let span: Span = cx.span(node);

    let expression: Expression<'a> = cx.expr(node, "expression")?;

    Ok(TSExportAssignment::new(span, expression, cx.builder()))
}

pub fn read_ts_namespace_export_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSNamespaceExportDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let id: IdentifierName<'a> = cx.req(node, "id", |cx, id_node| {
        Ok(IdentifierName::new(
            cx.span(id_node),
            cx.name(id_node)?,
            cx.builder(),
        ))
    })?;

    Ok(TSNamespaceExportDeclaration::new(span, id, cx.builder()))
}
