use std::cell::RefCell;

use crate::reader::json::Value;
use oxc::allocator::{
    Allocator, Box as ArenaBox, GetAllocator, Vec as ArenaVec,
};
use oxc::ast::ast::{
    BindingPattern, Comment, Directive, Expression, FunctionBody, Hashbang,
    Program, PropertyKey, SourceType, Statement, StringLiteral, TSType,
    TSTypeAnnotation, TSTypeParameterDeclaration, TSTypeParameterInstantiation,
};
use oxc::ast::builder::AstBuilder;
use oxc::span::Span;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use crate::errors::read::ReadError;

use super::literal;
use super::statement;
use super::ts_types;

pub struct ProgramReader<'a> {
    pub builder: AstBuilder<'a>,
}

impl<'a> ProgramReader<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        let builder: AstBuilder<'a> = AstBuilder::new(allocator);
        Self { builder }
    }

    pub fn read(
        &self,
        json: &Value,
        source_type: SourceType,
        source_text: &'a str,
    ) -> Result<Program<'a>, ReadError> {
        let cx: Cx<'a> = Cx::new(self);

        let mut body: ArenaVec<'a, Statement<'a>> =
            ArenaVec::new_in(&self.builder);

        let mut directives: ArenaVec<'a, Directive<'a>> =
            ArenaVec::new_in(&self.builder);

        cx.child(Seg::Field("body"), |cx| {
            split_directives_and_statements(
                cx,
                json,
                &mut directives,
                &mut body,
            )
        })?;

        let hashbang: Option<Hashbang<'a>> = match json
            .get("hashbang")
            .filter(|h| !h.is_null())
        {
            | Some(hashbang_node) => Some(read_hashbang(&cx, hashbang_node)?),
            | None => None,
        };

        let comments: ArenaVec<'a, Comment> = ArenaVec::new_in(&self.builder);

        let source_type: SourceType = resolve_source_type(source_type, json);

        let program: Program<'a> = Program::new(
            cx.span(json),
            source_type,
            source_text,
            comments,
            hashbang,
            directives,
            body,
            &self.builder,
        );

        Ok(program)
    }
}

fn resolve_source_type(
    source_type: SourceType,
    json: &Value,
) -> SourceType {
    if !source_type.is_unambiguous() {
        return source_type;
    }

    match json.get("sourceType").and_then(Value::as_str) {
        | Some("script") => source_type.with_script(true),
        | Some("commonjs") => source_type.with_commonjs(true),
        | _ => source_type.with_module(true),
    }
}

fn directive_if_any<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Option<Directive<'a>>, ReadError> {
    let is_directive: bool = ty_of(node) == "ExpressionStatement"
        && node.get("directive").is_some_and(Value::is_str);

    if !is_directive {
        return Ok(None);
    }

    let directive_str: &str =
        node.get("directive").and_then(Value::as_str).unwrap_or_default();

    let directive_atom: oxc::str::Str<'a> =
        oxc::str::Str::from_str_in(directive_str, cx.builder());

    let expression_node: &Value =
        node.get("expression").ok_or_else(|| cx.err(node))?;

    let expression: StringLiteral<'a> = cx
        .child(Seg::Field("expression"), |cx| {
            literal::read_string_literal(cx, expression_node)
        })?;

    let directive: Directive<'a> =
        Directive::new(cx.span(node), expression, directive_atom, cx.builder());

    Ok(Some(directive))
}

pub fn split_directives_and_statements<'a>(
    cx: &Cx<'a>,
    parent_node: &Value,
    directives: &mut ArenaVec<'a, Directive<'a>>,
    statements: &mut ArenaVec<'a, Statement<'a>>,
) -> Result<(), ReadError> {
    for (index, node) in parent_node
        .get("body")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        cx.push(Seg::Index(index));

        let step: Result<(), ReadError> = match directive_if_any(cx, node)? {
            | Some(directive) => {
                directives.push(directive);
                Ok(())
            },
            | None => {
                statements.push(statement::read_statement(cx, node)?);
                Ok(())
            },
        };

        cx.pop();

        step?;
    }

    Ok(())
}

pub fn read_function_body<'a>(
    cx: &Cx<'a>,
    body_node: &Value,
) -> Result<FunctionBody<'a>, ReadError> {
    let mut directives: ArenaVec<'a, Directive<'a>> =
        ArenaVec::new_in(cx.builder());

    let mut statements: ArenaVec<'a, Statement<'a>> =
        ArenaVec::new_in(cx.builder());

    cx.child(Seg::field("body"), |cx| {
        split_directives_and_statements(
            cx,
            body_node,
            &mut directives,
            &mut statements,
        )
    })?;

    let function_body: FunctionBody<'a> = FunctionBody::new(
        cx.span(body_node),
        directives,
        statements,
        cx.builder(),
    );

    Ok(function_body)
}

fn read_hashbang<'a>(
    cx: &Cx<'a>,
    node: &Value,
) -> Result<Hashbang<'a>, ReadError> {
    let value: &str =
        node.get("value").and_then(Value::as_str).unwrap_or_default();

    let value_atom: oxc::str::Str<'a> =
        oxc::str::Str::from_str_in(value, cx.builder());

    let hashbang: Hashbang<'a> =
        Hashbang::new(cx.span(node), value_atom, cx.builder());

    Ok(hashbang)
}

#[derive(Clone, Copy)]
pub enum Seg {
    Field(&'static str),
    Index(usize),
}

impl Seg {
    pub fn field(name: &'static str) -> Seg {
        Seg::Field(name)
    }

    pub fn index(value: usize) -> Seg {
        Seg::Index(value)
    }
}

pub struct Cx<'a> {
    pub builder: AstBuilder<'a>,
    trail: RefCell<Vec<Seg>>,
}

macro_rules! req_readers {
    ($($name:ident => $reader:path : $out:ty;)*) => {
        $(
            pub fn $name(
                &self,
                node: &Value,
                field: &'static str,
            ) -> Result<$out, ReadError> {
                self.req(node, field, $reader)
            }
        )*
    };
}

macro_rules! opt_readers {
    ($($name:ident => $reader:path : $out:ty;)*) => {
        $(
            pub fn $name(
                &self,
                node: &Value,
                field: &'static str,
            ) -> Result<Option<$out>, ReadError> {
                self.opt(node, field, $reader)
            }
        )*
    };
}

macro_rules! list_readers {
    ($($name:ident => $reader:path : $out:ty;)*) => {
        $(
            pub fn $name(
                &self,
                node: &Value,
                field: &'static str,
            ) -> Result<ArenaVec<'a, $out>, ReadError> {
                self.list(node, field, $reader)
            }
        )*
    };
}

impl<'a> Cx<'a> {
    fn new(reader: &ProgramReader<'a>) -> Cx<'a> {
        let allocator: &'a Allocator = reader.builder.allocator();

        let builder: AstBuilder<'a> = AstBuilder::new(allocator);

        let trail: RefCell<Vec<Seg>> =
            RefCell::new(vec![Seg::Field("Program")]);

        Cx { builder, trail }
    }

    pub fn span(
        &self,
        node: &Value,
    ) -> Span {
        let start: u32 =
            node.get("start").and_then(Value::as_u64).unwrap_or(0) as u32;

        let end_value: u64 =
            node.get("end").and_then(Value::as_u64).unwrap_or(u64::from(start));

        let end: u32 = end_value as u32;

        Span::new(start, end)
    }

    pub fn builder(&self) -> &AstBuilder<'a> {
        &self.builder
    }

    pub fn box_in<T>(
        &self,
        value: T,
    ) -> ArenaBox<'a, T> {
        ArenaBox::new_in(value, &self.builder)
    }

    pub fn err(
        &self,
        node: &Value,
    ) -> ReadError {
        let ty: &str = node
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("<missing type>");

        ReadError::from_message(format!(
            "unsupported ESTree node type `{ty}` at {}",
            self.path_string()
        ))
    }

    pub fn path_string(&self) -> String {
        let trail: Vec<Seg> = self.trail.borrow().clone();

        let mut path: String = String::new();

        for (index, seg) in trail.iter().enumerate() {
            match seg {
                | Seg::Field(name) => {
                    if index > 0 {
                        path.push('.');
                    }

                    path.push_str(name);
                },
                | Seg::Index(value) => {
                    path.push('[');

                    let digits: String = value.to_string();

                    path.push_str(&digits);

                    path.push(']');
                },
            }
        }

        path
    }

    pub fn push(
        &self,
        seg: Seg,
    ) {
        self.trail.borrow_mut().push(seg);
    }

    pub fn pop(&self) {
        self.trail.borrow_mut().pop();
    }

    pub fn child<T, F>(
        &self,
        seg: Seg,
        f: F,
    ) -> Result<T, ReadError>
    where
        F: FnOnce(&Cx<'a>) -> Result<T, ReadError>,
    {
        self.push(seg);

        let result: Result<T, ReadError> = f(self);

        self.pop();

        result
    }

    pub fn req<T, F>(
        &self,
        node: &Value,
        field: &'static str,
        f: F,
    ) -> Result<T, ReadError>
    where
        F: FnOnce(&Cx<'a>, &Value) -> Result<T, ReadError>,
    {
        let child_node: &Value =
            node.get(field).ok_or_else(|| self.err(node))?;

        self.push(Seg::Field(field));

        let result: Result<T, ReadError> = f(self, child_node);

        self.pop();

        result
    }

    pub fn opt<T, F>(
        &self,
        node: &Value,
        field: &'static str,
        f: F,
    ) -> Result<Option<T>, ReadError>
    where
        F: FnOnce(&Cx<'a>, &Value) -> Result<T, ReadError>,
    {
        match node.get(field) {
            | Some(child_node) if !child_node.is_null() => {
                self.push(Seg::Field(field));
                let result: Result<T, ReadError> = f(self, child_node);
                self.pop();
                result.map(Some)
            },
            | _ => Ok(None),
        }
    }

    pub fn list<T, F>(
        &self,
        node: &Value,
        field: &'static str,
        mut f: F,
    ) -> Result<ArenaVec<'a, T>, ReadError>
    where
        F: FnMut(&Cx<'a>, &Value) -> Result<T, ReadError>,
    {
        let mut items: ArenaVec<'a, T> = ArenaVec::new_in(&self.builder);

        self.push(Seg::Field(field));

        for (index, item) in node
            .get(field)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            self.push(Seg::Index(index));

            let result: Result<T, ReadError> = f(self, item);

            self.pop();

            items.push(result?);
        }

        self.pop();

        Ok(items)
    }

    pub fn list_opt<T, F>(
        &self,
        node: &Value,
        field: &'static str,
        f: F,
    ) -> Result<Option<ArenaVec<'a, T>>, ReadError>
    where
        F: FnMut(&Cx<'a>, &Value) -> Result<T, ReadError>,
    {
        match node.get(field).and_then(Value::as_array) {
            | Some(items) if !items.is_empty() => {
                Ok(Some(self.list(node, field, f)?))
            },
            | _ => Ok(None),
        }
    }

    pub fn opt_box<T, F>(
        &self,
        node: &Value,
        field: &'static str,
        f: F,
    ) -> Result<Option<ArenaBox<'a, T>>, ReadError>
    where
        F: FnOnce(&Cx<'a>, &Value) -> Result<T, ReadError>,
    {
        self.opt(node, field, |cx, child| Ok(cx.box_in(f(cx, child)?)))
    }

    pub fn flag(
        &self,
        node: &Value,
        field: &'static str,
    ) -> bool {
        node.get(field).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn flag_or(
        &self,
        node: &Value,
        field: &'static str,
        default: bool,
    ) -> bool {
        node.get(field).and_then(Value::as_bool).unwrap_or(default)
    }

    pub fn name(
        &self,
        node: &Value,
    ) -> Result<oxc::str::Ident<'a>, ReadError> {
        let name_node: &Value =
            node.get("name").ok_or_else(|| self.err(node))?;

        let name: &str = name_node.as_str().unwrap_or_default();

        Ok(oxc::str::Ident::from_str_in(name, &self.builder))
    }

    pub fn text(
        &self,
        node: &Value,
        field: &'static str,
    ) -> oxc::str::Str<'a> {
        let value: &str =
            node.get(field).and_then(Value::as_str).unwrap_or_default();

        oxc::str::Str::from_str_in(value, &self.builder)
    }

    req_readers! {
        expr => super::expression::read_expression : Expression<'a>;
        stmt => statement::read_statement : Statement<'a>;
        pattern => super::pattern::read_binding_pattern :
            BindingPattern<'a>;
        ts => ts_types::read_ts_type : TSType<'a>;
        string_literal => literal::read_string_literal :
            StringLiteral<'a>;
        property_key => super::pattern::read_property_key :
            PropertyKey<'a>;
    }

    opt_readers! {
        opt_expr => super::expression::read_expression :
            Expression<'a>;
        opt_stmt => statement::read_statement : Statement<'a>;
        opt_ts => ts_types::read_ts_type : TSType<'a>;
    }

    list_readers! {
        exprs => super::expression::read_expression :
            Expression<'a>;
        stmts => statement::read_statement : Statement<'a>;
        ts_types => ts_types::read_ts_type : TSType<'a>;
    }

    pub fn opt_type_arguments(
        &self,
        node: &Value,
    ) -> Result<Option<ArenaBox<'a, TSTypeParameterInstantiation<'a>>>, ReadError>
    {
        self.opt(
            node,
            "typeArguments",
            ts_types::read_ts_type_parameter_instantiation,
        )
        .map(|instantiation| instantiation.map(|value| self.box_in(value)))
    }

    pub fn opt_type_parameters(
        &self,
        node: &Value,
    ) -> Result<Option<ArenaBox<'a, TSTypeParameterDeclaration<'a>>>, ReadError>
    {
        self.opt(
            node,
            "typeParameters",
            ts_types::read_ts_type_parameter_declaration,
        )
    }

    pub fn opt_annotation(
        &self,
        node: &Value,
    ) -> Result<Option<ArenaBox<'a, TSTypeAnnotation<'a>>>, ReadError> {
        self.opt(node, "typeAnnotation", ts_types::read_ts_type_annotation)
            .map(|annotation| annotation.flatten())
    }

    pub fn annotation(
        &self,
        node: &Value,
        field: &'static str,
    ) -> Result<Option<ArenaBox<'a, TSTypeAnnotation<'a>>>, ReadError> {
        self.opt(node, field, ts_types::read_ts_type_annotation)
            .map(|annotation| annotation.flatten())
    }
}

pub fn ty_of(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or("")
}

macro_rules! nodes {
    (
        $cx:ident, $node:ident, $ty:expr, $out:ident :
        $( $ty_lit:literal => $variant:ident : $ctor:path [ $($arg:expr),* $(,)? ] ; )*
    ) => {
        match $ty {
            $(
                $ty_lit => {
                    let value = $ctor($($arg,)* &$cx.builder);
                    Ok($out::$variant($cx.box_in(value)))
                },
            )*
            _ => Err($cx.err($node)),
        }
    };
}

pub(crate) use nodes;
