use std::cell::RefCell;

use oxc::allocator::{
    Allocator, Box as ArenaBox, GetAllocator, Vec as ArenaVec,
};
use oxc::ast::ast::{
    BindingPattern, Expression, PropertyKey, Statement, StringLiteral, TSType,
    TSTypeAnnotation, TSTypeParameterDeclaration, TSTypeParameterInstantiation,
};
use oxc::ast::builder::AstBuilder;
use oxc::span::Span;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use crate::errors::read::ReadError;
use crate::reader::engine::program::ProgramReader;
use crate::reader::expression::expressions;
use crate::reader::json::Value;
use crate::reader::literal;
use crate::reader::pattern::binding;
use crate::reader::pattern::targets;
use crate::reader::statement;
use crate::reader::ts_types::types;

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

pub fn ty_of(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or("")
}

fn ty_label(node: &Value) -> String {
    let ty: &str = ty_of(node);

    if ty.is_empty() { "<missing type>".to_string() } else { ty.to_string() }
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
    pub fn new(reader: &ProgramReader<'a>) -> Cx<'a> {
        let allocator: &'a Allocator = reader.builder.allocator();

        let builder: AstBuilder<'a> = AstBuilder::new(allocator);

        let trail: RefCell<Vec<Seg>> =
            RefCell::new(vec![Seg::Field("Program")]);

        Cx { builder, trail }
    }

    // Spans are optional in some ESTree producers; absent `start`/`end`
    // degrade to 0/0 rather than failing the read.
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
        ReadError::NodeUnsupported {
            ty: ty_label(node),
            path: self.path_string(),
        }
    }

    pub fn missing(
        &self,
        node: &Value,
        field: &'static str,
    ) -> ReadError {
        ReadError::FieldMissing {
            field,
            ty: ty_label(node),
            path: self.path_string(),
        }
    }

    pub fn invalid(
        &self,
        node: &Value,
        field: &'static str,
        expected: &'static str,
    ) -> ReadError {
        ReadError::FieldInvalid {
            field,
            expected,
            ty: ty_label(node),
            path: self.path_string(),
        }
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
            node.get(field).ok_or_else(|| self.missing(node, field))?;

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

    // Legacy ESTree omits boolean flags that are false; absence reads as `false`.
    pub fn flag(
        &self,
        node: &Value,
        field: &'static str,
    ) -> bool {
        node.get(field).and_then(Value::as_bool).unwrap_or(false)
    }

    // Legacy ESTree omits boolean flags that are false; absence reads as `false`.
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
            node.get("name").ok_or_else(|| self.missing(node, "name"))?;

        let name: &str = name_node
            .as_str()
            .ok_or_else(|| self.invalid(node, "name", "a string"))?;

        Ok(oxc::str::Ident::from_str_in(name, &self.builder))
    }

    pub fn text(
        &self,
        node: &Value,
        field: &'static str,
    ) -> Result<oxc::str::Str<'a>, ReadError> {
        let value_node: &Value =
            node.get(field).ok_or_else(|| self.missing(node, field))?;

        let value: &str = value_node
            .as_str()
            .ok_or_else(|| self.invalid(node, field, "a string"))?;

        Ok(oxc::str::Str::from_str_in(value, &self.builder))
    }

    req_readers! {
        expr => expressions::read_expression :
            Expression<'a>;
        stmt => statement::read_statement : Statement<'a>;
        pattern => binding::read_binding_pattern :
            BindingPattern<'a>;
        ts => types::read_ts_type : TSType<'a>;
        string_literal => literal::read_string_literal :
            StringLiteral<'a>;
        property_key => targets::read_property_key :
            PropertyKey<'a>;
    }

    opt_readers! {
        opt_expr => expressions::read_expression :
            Expression<'a>;
        opt_stmt => statement::read_statement : Statement<'a>;
        opt_ts => types::read_ts_type : TSType<'a>;
    }

    list_readers! {
        exprs => expressions::read_expression :
            Expression<'a>;
        stmts => statement::read_statement : Statement<'a>;
        ts_types => types::read_ts_type : TSType<'a>;
    }

    pub fn opt_type_arguments(
        &self,
        node: &Value,
    ) -> Result<Option<ArenaBox<'a, TSTypeParameterInstantiation<'a>>>, ReadError>
    {
        self.opt(
            node,
            "typeArguments",
            types::read_ts_type_parameter_instantiation,
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
            types::read_ts_type_parameter_declaration,
        )
    }

    pub fn annotation(
        &self,
        node: &Value,
        field: &'static str,
    ) -> Result<Option<ArenaBox<'a, TSTypeAnnotation<'a>>>, ReadError> {
        self.opt(node, field, types::read_ts_type_annotation)
            .map(|annotation| annotation.flatten())
    }
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
