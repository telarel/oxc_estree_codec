use crate::reader::json::Value;
use oxc::allocator::{CloneIn, GetAllocator};
use oxc::ast::ast::{
    AccessorProperty, AccessorPropertyType, BindingIdentifier, BindingPattern,
    BindingRestElement, Class, ClassBody, ClassElement, ClassHeritage,
    ClassType, Declaration, Decorator, ExportAllDeclaration, ExportDeclaration,
    ExportDefaultDeclaration, ExportDefaultDeclarationKind,
    ExportFromDeclaration, ExportNamedDeclaration, ExportSpecifier, Expression,
    FormalParameter, FormalParameterKind, FormalParameterRest,
    FormalParameters, Function, FunctionBody, FunctionType, IdentifierName,
    IdentifierReference, ImportAttribute, ImportAttributeKey,
    ImportDeclaration, ImportDeclarationSpecifier, ImportDefaultSpecifier,
    ImportNamespaceSpecifier, ImportOrExportKind, ImportPhase, ImportSpecifier,
    MethodDefinition, MethodDefinitionKind, MethodDefinitionType,
    ModuleExportName, PropertyDefinition, PropertyDefinitionType, PropertyKey,
    Statement, StaticBlock, StringLiteral, TSAccessibility, TSClassImplements,
    TSEnumBody, TSEnumDeclaration, TSEnumMember, TSEnumMemberName,
    TSExportAssignment, TSExternalModuleDeclaration, TSExternalModuleReference,
    TSGlobalDeclaration, TSImportEqualsDeclaration, TSInterfaceBody,
    TSInterfaceDeclaration, TSInterfaceHeritage, TSModuleBlock,
    TSModuleReference, TSNamespaceDeclaration, TSNamespaceDeclarationBody,
    TSNamespaceDeclarationKind, TSQualifiedName, TSThisParameter, TSType,
    TSTypeAliasDeclaration, TSTypeAnnotation, TSTypeName, WithClause,
    WithClauseKeyword,
};
use oxc::span::Span;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use super::engine::{Cx, Seg, ty_of};
use super::expression;
use super::literal;
use super::pattern;
use super::statement;
use super::ts_types;
use crate::errors::read::ReadError;

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
                super::engine::read_function_body(cx, body_node)?;
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
                let argument: BindingPattern<'a> =
                    cx.req(item, "argument", pattern::read_binding_pattern)?;

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
                        pattern::read_binding_pattern(cx, left_node)?;

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
                pattern::read_binding_pattern(cx, parameter_node)?;

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
                pattern::read_binding_pattern(cx, left_node)?;

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

    let pattern: BindingPattern<'a> = pattern::read_binding_pattern(cx, item)?;

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

pub fn read_decorators<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<oxc::allocator::Vec<'a, Decorator<'a>>, ReadError> {
    cx.list(node, "decorators", |cx, item| {
        let expression_node: &Value =
            item.get("expression").ok_or_else(|| cx.err(item))?;

        let expression: Expression<'a> = cx
            .child(Seg::field("expression"), |cx| {
                expression::read_expression(cx, expression_node)
            })?;

        Ok(Decorator::new(cx.span(item), expression, cx.builder()))
    })
}

fn accessibility<'a>(
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
                expression: expression::read_expression(cx, super_node)?,
                type_arguments: cx.opt_box(
                    node,
                    "superTypeArguments",
                    ts_types::read_ts_type_parameter_instantiation,
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

fn read_class_implements<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<oxc::allocator::Vec<'a, TSClassImplements<'a>>, ReadError> {
    cx.list(node, "implements", |cx, item| {
        let expression_node: &Value =
            item.get("expression").ok_or_else(|| cx.err(item))?;

        let expression: TSTypeName<'a> =
            cx.child(Seg::field("expression"), |cx| {
                ts_types::read_ts_type_name_from_expression(cx, expression_node)
            })?;

        let implements_entry: TSClassImplements<'a> = TSClassImplements::new(
            cx.span(item),
            expression,
            cx.opt_box(
                item,
                "typeArguments",
                ts_types::read_ts_type_parameter_instantiation,
            )?,
            cx.builder(),
        );

        Ok(implements_entry)
    })
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
        | _ => Err(cx.err(node)),
    }
}

fn method_definition<'a>(
    cx: &Cx<'a>,
    node: &Value,
    ty: &str,
) -> Result<MethodDefinition<'a>, ReadError> {
    let span: Span = cx.span(node);

    let key: PropertyKey<'a> = cx.property_key(node, "key")?;

    let function: Function<'a> = cx.req(node, "value", |cx, value_node| {
        read_function(cx, value_node, FunctionType::FunctionExpression)
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

pub fn read_class_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Class<'a>, ReadError> {
    read_class(cx, node, ClassType::ClassDeclaration)
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

fn local_binding_identifier<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingIdentifier<'a>, ReadError> {
    cx.req(node, "local", read_binding_identifier_at)
}

pub fn read_binding_identifier_at<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<BindingIdentifier<'a>, ReadError> {
    Ok(BindingIdentifier::new(cx.span(node), cx.name(node)?, cx.builder()))
}

pub fn read_binding_identifier_field<'a>(
    cx: &Cx<'a>,
    node: &Value,
    field: &'static str,
) -> Result<BindingIdentifier<'a>, ReadError> {
    cx.req(node, field, read_binding_identifier_at)
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
            | "ClassDeclaration" => {
                Ok(ExportDefaultDeclarationKind::ClassDeclaration(
                    cx.box_in(read_class_declaration(cx, declaration_node)?),
                ))
            },
            | "TSInterfaceDeclaration" => Err(cx.err(declaration_node)),
            | _ => Ok(ExportDefaultDeclarationKind::from(
                expression::read_expression(cx, declaration_node)?,
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

pub fn read_ts_type_alias_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSTypeAliasDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let type_annotation: TSType<'a> = cx.ts(node, "typeAnnotation")?;

    Ok(TSTypeAliasDeclaration::new(
        span,
        read_binding_identifier_field(cx, node, "id")?,
        cx.opt_type_parameters(node)?,
        type_annotation,
        cx.flag(node, "declare"),
        cx.builder(),
    ))
}

pub fn read_ts_interface_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSInterfaceDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let extends: oxc::allocator::Vec<'a, TSInterfaceHeritage<'a>> =
        cx.list(node, "extends", ts_interface_heritage)?;

    let body: TSInterfaceBody<'a> =
        cx.req(node, "body", |cx: &Cx<'a>, body_node: &Value| {
            Ok(TSInterfaceBody::new(
                cx.span(body_node),
                ts_types::read_ts_signatures(cx, body_node, "body")?,
                cx.builder(),
            ))
        })?;

    Ok(TSInterfaceDeclaration::new(
        span,
        read_binding_identifier_field(cx, node, "id")?,
        cx.opt_type_parameters(node)?,
        extends,
        cx.box_in(body),
        cx.flag(node, "declare"),
        cx.builder(),
    ))
}

fn ts_interface_heritage<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSInterfaceHeritage<'a>, ReadError> {
    let expression: TSTypeName<'a> =
        cx.req(node, "expression", |cx, expression_node| {
            ts_types::read_ts_type_name_from_expression(cx, expression_node)
        })?;

    Ok(TSInterfaceHeritage::new(
        cx.span(node),
        expression,
        cx.opt_box(
            node,
            "typeArguments",
            ts_types::read_ts_type_parameter_instantiation,
        )?,
        cx.builder(),
    ))
}

pub fn read_ts_enum_declaration<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSEnumDeclaration<'a>, ReadError> {
    let span: Span = cx.span(node);

    let body: TSEnumBody<'a> =
        cx.req(node, "body", |cx: &Cx<'a>, body_node: &Value| {
            Ok(TSEnumBody::new(
                cx.span(body_node),
                cx.list(body_node, "members", ts_enum_member)?,
                cx.builder(),
            ))
        })?;

    Ok(TSEnumDeclaration::new(
        span,
        read_binding_identifier_field(cx, node, "id")?,
        body,
        cx.flag(node, "const"),
        cx.flag(node, "declare"),
        cx.builder(),
    ))
}

fn ts_enum_member<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSEnumMember<'a>, ReadError> {
    let span: Span = cx.span(node);

    let name: TSEnumMemberName<'a> = cx.req(node, "id", ts_enum_member_name)?;

    Ok(TSEnumMember::new(
        span,
        name,
        cx.opt_expr(node, "initializer")?,
        cx.builder(),
    ))
}

fn ts_enum_member_name<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSEnumMemberName<'a>, ReadError> {
    match ty_of(node) {
        | "Identifier" => Ok(TSEnumMemberName::Identifier(cx.box_in(
            IdentifierName::new(cx.span(node), cx.name(node)?, cx.builder()),
        ))),
        | "Literal" => {
            let literal_node: StringLiteral<'a> =
                literal::read_string_literal(cx, node)?;

            let name: TSEnumMemberName<'a> = if cx.flag(node, "computed") {
                TSEnumMemberName::ComputedString(cx.box_in(literal_node))
            } else {
                TSEnumMemberName::String(cx.box_in(literal_node))
            };

            Ok(name)
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

    let kind: &str = node.get("kind").and_then(Value::as_str).unwrap_or("");

    let body: Option<oxc::allocator::Box<'a, TSModuleBlock<'a>>> =
        cx.opt_box(node, "body", ts_module_block)?;

    let id_node: &Value = node.get("id").ok_or_else(|| cx.err(node))?;

    cx.child(Seg::field("id"), |cx| match ty_of(id_node) {
        | "Identifier" => {
            let id: BindingIdentifier<'a> =
                ts_types::read_binding_identifier(cx, id_node)?;
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

fn read_module_name_parts<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Vec<BindingIdentifier<'a>>, ReadError> {
    match ty_of(node) {
        | "Identifier" => {
            Ok(vec![ts_types::read_binding_identifier(cx, node)?])
        },
        | "TSQualifiedName" => {
            let mut parts: Vec<BindingIdentifier<'a>> =
                cx.req(node, "left", read_module_name_parts)?;
            parts.push(cx.req(
                node,
                "right",
                ts_types::read_binding_identifier,
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

    let body: oxc::allocator::Vec<'a, Statement<'a>> =
        cx.stmts(node, "body")?;

    Ok(TSModuleBlock::new(
        span,
        oxc::allocator::Vec::new_in(cx.builder()),
        body,
        cx.builder(),
    ))
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
            let left: TSTypeName<'a> =
                cx.req(node, "left", ts_types::read_ts_type_name)?;

            let right: IdentifierName<'a> =
                cx.req(node, "right", ts_types::read_identifier_name)?;

            let qualified: TSQualifiedName<'a> =
                TSQualifiedName::new(cx.span(node), left, right, cx.builder());

            Ok(TSModuleReference::QualifiedName(cx.box_in(qualified)))
        },
        | _ => Err(cx.err(node)),
    }
}

pub fn read_ts_export_assignment<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<TSExportAssignment<'a>, ReadError> {
    let span: Span = cx.span(node);

    let expression: Expression<'a> = cx.expr(node, "expression")?;

    Ok(TSExportAssignment::new(span, expression, cx.builder()))
}
