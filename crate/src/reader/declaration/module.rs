use oxc::allocator::{CloneIn, GetAllocator};
use oxc::ast::ast::{
    BindingIdentifier, Directive, IdentifierName, IdentifierReference,
    Statement, StringLiteral, TSExternalModuleDeclaration,
    TSExternalModuleReference, TSGlobalDeclaration, TSImportEqualsDeclaration,
    TSModuleBlock, TSModuleReference, TSNamespaceDeclaration,
    TSNamespaceDeclarationBody, TSNamespaceDeclarationKind, TSQualifiedName,
    TSTypeName,
};
use oxc::span::Span;
use sonic_rs::JsonValueTrait;

use crate::errors::read::ReadError;
use crate::reader::declaration::function::read_binding_identifier_field;
use crate::reader::declaration::import_export::read_import_or_export_kind;
use crate::reader::engine::context::{Cx, Seg, ty_of};
use crate::reader::engine::program::split_directives_and_statements;
use crate::reader::json::Value;
use crate::reader::literal;

fn read_module_name_parts<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Vec<BindingIdentifier<'a>>, ReadError> {
    match ty_of(node) {
        | "Identifier" => {
            Ok(vec![crate::reader::ts_types::names::read_binding_identifier(
                cx, node,
            )?])
        },
        | "TSQualifiedName" => {
            let mut parts: Vec<BindingIdentifier<'a>> =
                cx.req(node, "left", read_module_name_parts)?;
            parts.push(cx.req(
                node,
                "right",
                crate::reader::ts_types::names::read_binding_identifier,
            )?);
            Ok(parts)
        },
        | _ => Err(cx.err(node)),
    }
}

fn ts_module_block<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSModuleBlock<'a>, ReadError> {
    let span: Span = cx.span(node);

    // directives on the module block are split out like program-level
    // directives; oxc stores them on the block itself.
    let mut directives: oxc::allocator::Vec<'a, Directive<'a>> =
        oxc::allocator::Vec::new_in(cx.builder());

    let mut body: oxc::allocator::Vec<'a, Statement<'a>> =
        oxc::allocator::Vec::new_in(cx.builder());

    cx.child(Seg::field("body"), |cx| {
        split_directives_and_statements(cx, node, &mut directives, &mut body)
    })?;

    Ok(TSModuleBlock::new(span, directives, body, cx.builder()))
}

fn ts_module_reference<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSModuleReference<'a>, ReadError> {
    match ty_of(node) {
        | "TSExternalModuleReference" => {
            let expression: StringLiteral<'a> =
                cx.req(node, "expression", |cx, expression_node| {
                    literal::read_string_literal(cx, expression_node)
                })?;

            let reference: TSExternalModuleReference<'a> =
                TSExternalModuleReference::new(
                    cx.span(node),
                    expression,
                    cx.builder(),
                );

            Ok(TSModuleReference::ExternalModuleReference(cx.box_in(reference)))
        },
        | "Identifier" => {
            let reference: IdentifierReference<'a> = IdentifierReference::new(
                cx.span(node),
                cx.name(node)?,
                cx.builder(),
            );

            Ok(TSModuleReference::IdentifierReference(cx.box_in(reference)))
        },
        | "TSQualifiedName" => {
            let left: TSTypeName<'a> = cx.req(
                node,
                "left",
                crate::reader::ts_types::names::read_ts_type_name,
            )?;

            let right: IdentifierName<'a> = cx.req(
                node,
                "right",
                crate::reader::ts_types::names::read_identifier_name,
            )?;

            let qualified: TSQualifiedName<'a> =
                TSQualifiedName::new(cx.span(node), left, right, cx.builder());

            Ok(TSModuleReference::QualifiedName(cx.box_in(qualified)))
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_ts_module_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Statement<'a>, ReadError> {
    let span: Span = cx.span(node);

    let declare: bool = cx.flag(node, "declare");

    let kind_node: &Value =
        node.get("kind").ok_or_else(|| cx.missing(node, "kind"))?;

    let kind: &str = kind_node
        .as_str()
        .ok_or_else(|| cx.invalid(node, "kind", "a string"))?;

    let body: Option<oxc::allocator::Box<'a, TSModuleBlock<'a>>> =
        cx.opt_box(node, "body", ts_module_block)?;

    let id_node: &Value = node.get("id").ok_or_else(|| cx.err(node))?;

    cx.child(Seg::field("id"), |cx| match ty_of(id_node) {
        | "Identifier" => {
            let id: BindingIdentifier<'a> =
                crate::reader::ts_types::names::read_binding_identifier(
                    cx, id_node,
                )?;
            if kind == "global" {
                let block: TSModuleBlock<'a> = match body {
                    | Some(block) => block.unbox(),
                    | None => return Err(cx.err(node)),
                };

                let global_span: Span = cx.span(id_node);

                let declaration: TSGlobalDeclaration<'a> =
                    TSGlobalDeclaration::new(
                        span,
                        global_span,
                        block,
                        declare,
                        cx.builder(),
                    );

                Ok(Statement::TSGlobalDeclaration(cx.box_in(declaration)))
            } else {
                let declaration_kind: TSNamespaceDeclarationKind = match kind {
                    | "namespace" => TSNamespaceDeclarationKind::Namespace,
                    | "module" => TSNamespaceDeclarationKind::Module,
                    | other => {
                        return Err(ReadError::ValueUnsupported {
                            kind: "module declaration kind",
                            value: other.to_string(),
                            path: cx.path_string(),
                        });
                    },
                };

                let declaration_body: TSNamespaceDeclarationBody<'a> =
                    match body {
                        | Some(block) => {
                            TSNamespaceDeclarationBody::TSModuleBlock(block)
                        },
                        | None => return Err(cx.err(node)),
                    };

                let declaration: TSNamespaceDeclaration<'a> =
                    TSNamespaceDeclaration::new(
                        span,
                        id,
                        declaration_body,
                        declaration_kind,
                        declare,
                        cx.builder(),
                    );

                Ok(Statement::TSNamespaceDeclaration(cx.box_in(declaration)))
            }
        },
        | "TSQualifiedName" => {
            let parts: Vec<BindingIdentifier<'a>> =
                read_module_name_parts(cx, id_node)?;

            let block: TSModuleBlock<'a> = match body {
                | Some(block) => block.unbox(),
                | None => return Err(cx.err(node)),
            };

            let declaration_kind: TSNamespaceDeclarationKind = match kind {
                | "namespace" => TSNamespaceDeclarationKind::Namespace,
                | "module" => TSNamespaceDeclarationKind::Module,
                | other => {
                    return Err(ReadError::ValueUnsupported {
                        kind: "module declaration kind",
                        value: other.to_string(),
                        path: cx.path_string(),
                    });
                },
            };

            let mut current: Option<TSNamespaceDeclaration<'a>> = None;

            for (index, part) in parts.iter().enumerate().rev() {
                let part_body: TSNamespaceDeclarationBody<'a> =
                    match (index + 1 == parts.len(), current.take()) {
                        | (true, _) => {
                            TSNamespaceDeclarationBody::TSModuleBlock(
                                cx.box_in(
                                    block.clone_in(cx.builder().allocator()),
                                ),
                            )
                        },
                        | (false, Some(inner)) => {
                            TSNamespaceDeclarationBody::TSNamespaceDeclaration(
                                cx.box_in(inner),
                            )
                        },
                        | (false, None) => return Err(cx.err(node)),
                    };

                let declaration: TSNamespaceDeclaration<'a> =
                    TSNamespaceDeclaration::new(
                        span,
                        part.clone_in(cx.builder().allocator()),
                        part_body,
                        declaration_kind,
                        declare,
                        cx.builder(),
                    );

                current = Some(declaration);
            }

            let declaration: TSNamespaceDeclaration<'a> =
                current.ok_or_else(|| cx.err(node))?;

            Ok(Statement::TSNamespaceDeclaration(cx.box_in(declaration)))
        },
        | "Literal" => {
            let id: StringLiteral<'a> =
                literal::read_string_literal(cx, id_node)?;

            let declaration: TSExternalModuleDeclaration<'a> =
                TSExternalModuleDeclaration::new(
                    span,
                    id,
                    body,
                    declare,
                    cx.builder(),
                );

            Ok(Statement::TSExternalModuleDeclaration(cx.box_in(declaration)))
        },
        | _ => Err(cx.err(id_node)),
    })
}

pub fn read_ts_import_equals_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSImportEqualsDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let reference: TSModuleReference<'a> =
        cx.req(node, "moduleReference", ts_module_reference)?;

    Ok(TSImportEqualsDeclaration::new(
        span,
        read_binding_identifier_field(cx, node, "id")?,
        reference,
        read_import_or_export_kind(
            cx,
            node.get("importKind").and_then(Value::as_str),
        )?,
        cx.builder(),
    ))
}
