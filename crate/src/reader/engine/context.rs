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
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};

use crate::_types::options::SpanEncoding;
use crate::errors::read::ReadError;
use crate::reader::engine::program::ProgramReader;
use crate::reader::expression::expressions;
use crate::reader::json;
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

// Sparse UTF-16 code unit to UTF-8 byte offset map. Only non-ASCII
// characters need an entry: ASCII advances both units and bytes equally.
struct Utf16Char {
    utf16_start: u32,
    utf16_len: u32,
    byte_start: u32,
    byte_len: u32,
}

struct Utf16Map {
    chars: Vec<Utf16Char>,
}

impl Utf16Map {
    fn new(source_text: &str) -> Option<Utf16Map> {
        if source_text.is_ascii() {
            return None;
        }

        let mut chars: Vec<Utf16Char> = Vec::new();
        let mut utf16_start: u32 = 0;

        for (byte_start, ch) in source_text.char_indices() {
            let utf16_len: u32 = ch.len_utf16() as u32;
            let byte_len: u32 = ch.len_utf8() as u32;

            if !ch.is_ascii() {
                chars.push(Utf16Char {
                    utf16_start,
                    utf16_len,
                    byte_start: byte_start as u32,
                    byte_len,
                });
            }

            utf16_start += utf16_len;
        }

        Some(Utf16Map { chars })
    }

    fn to_byte(
        &self,
        offset: u32,
    ) -> Option<u32> {
        let Some(first): Option<&Utf16Char> = self.chars.first() else {
            return Some(offset);
        };

        if offset < first.utf16_start {
            return Some(offset);
        }

        let index: usize =
            self.chars.partition_point(|entry| entry.utf16_start <= offset) - 1;

        let entry: &Utf16Char = &self.chars[index];

        let utf16_end: u32 = entry.utf16_start.checked_add(entry.utf16_len)?;

        if offset < utf16_end {
            // Inside a non-ASCII character only its leading boundary maps.
            return (offset == entry.utf16_start).then_some(entry.byte_start);
        }

        let delta: u32 = offset - utf16_end;

        entry.byte_start.checked_add(entry.byte_len)?.checked_add(delta)
    }
}

pub struct Cx<'a> {
    pub builder: AstBuilder<'a>,
    trail: RefCell<Vec<Seg>>,
    source_text: &'a str,
    span_encoding: SpanEncoding,
    utf16_map: Option<Utf16Map>,
    utf16_len: u64,
    span_error: RefCell<Option<ReadError>>,
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
    pub fn new(
        reader: &ProgramReader<'a>,
        source_text: &'a str,
        span_encoding: SpanEncoding,
    ) -> Cx<'a> {
        let allocator: &'a Allocator = reader.builder.allocator();

        let builder: AstBuilder<'a> = AstBuilder::new(allocator);

        let trail: RefCell<Vec<Seg>> =
            RefCell::new(vec![Seg::Field("Program")]);

        let (utf16_map, utf16_len): (Option<Utf16Map>, u64) =
            match span_encoding {
                | SpanEncoding::Utf8Bytes => (None, 0),
                | SpanEncoding::Utf16Units => (
                    Utf16Map::new(source_text),
                    source_text.encode_utf16().count() as u64,
                ),
            };

        Cx {
            builder,
            trail,
            source_text,
            span_encoding,
            utf16_map,
            utf16_len,
            span_error: RefCell::new(None),
        }
    }

    fn range_offsets(
        &self,
        node: &Value,
    ) -> Result<Option<(u64, u64)>, ReadError> {
        let Some(range_node): Option<&Value> = node.get("range") else {
            return Ok(None);
        };

        let range: &[Value] = range_node
            .as_array()
            .map(|values| values.as_slice())
            .ok_or_else(|| self.invalid(node, "range", "[start, end]"))?;

        let [start_node, end_node] = range else {
            return Err(self.invalid(node, "range", "[start, end]"));
        };

        let start: u64 = start_node
            .as_u64()
            .ok_or_else(|| self.invalid(node, "range", "[start, end]"))?;

        let end: u64 = end_node
            .as_u64()
            .ok_or_else(|| self.invalid(node, "range", "[start, end]"))?;

        Ok(Some((start, end)))
    }

    fn span_offsets(
        &self,
        node: &Value,
    ) -> Result<Option<(u64, u64)>, ReadError> {
        let start: Option<&Value> = node.get("start");
        let end: Option<&Value> = node.get("end");

        match (start, end) {
            | (None, None) => self.range_offsets(node),
            | (start, end) => {
                let start: u64 = start
                    .ok_or_else(|| self.invalid(node, "start", "a number"))?
                    .as_u64()
                    .ok_or_else(|| self.invalid(node, "start", "a number"))?;
                let end: u64 = end
                    .ok_or_else(|| self.invalid(node, "end", "a number"))?
                    .as_u64()
                    .ok_or_else(|| self.invalid(node, "end", "a number"))?;
                Ok(Some((start, end)))
            },
        }
    }

    fn utf16_to_byte(
        &self,
        node: &Value,
        field: &'static str,
        offset: u64,
    ) -> Result<u32, ReadError> {
        const EXPECTED: &str =
            "a UTF-16 offset on a UTF-8 char boundary within source_text";

        if offset > self.utf16_len {
            return Err(self.invalid(node, field, EXPECTED));
        }

        let offset: u32 = u32::try_from(offset)
            .map_err(|_| self.invalid(node, field, EXPECTED))?;

        match &self.utf16_map {
            | Some(map) => map.to_byte(offset),
            | None => Some(offset),
        }
        .ok_or_else(|| self.invalid(node, field, EXPECTED))
    }

    pub(crate) fn checked_span_opt(
        &self,
        node: &Value,
    ) -> Result<Option<Span>, ReadError> {
        let Some((start, end)) = self.span_offsets(node)? else {
            return Ok(None);
        };

        if end < start {
            return Err(self.invalid(
                node,
                "end",
                "an offset at or after start",
            ));
        }

        match self.span_encoding {
            | SpanEncoding::Utf8Bytes => {
                let start: u32 = u32::try_from(start).map_err(|_| {
                    self.invalid(node, "start", "an offset that fits in u32")
                })?;

                let end: u32 = u32::try_from(end).map_err(|_| {
                    self.invalid(node, "end", "an offset that fits in u32")
                })?;

                Ok(Some(Span::new(start, end)))
            },
            | SpanEncoding::Utf16Units => {
                let start: u32 = self.utf16_to_byte(node, "start", start)?;

                let end: u32 = self.utf16_to_byte(node, "end", end)?;

                Ok(Some(Span::new(start, end)))
            },
        }
    }

    pub(crate) fn source_text(&self) -> &'a str {
        self.source_text
    }

    // Spans are optional in some ESTree producers. When `start`/`end` and
    // `range` are absent, the span degrades to 0/0 rather than failing the
    // read.
    pub fn span(
        &self,
        node: &Value,
    ) -> Span {
        match self.checked_span_opt(node) {
            | Ok(Some(span)) => span,
            | Ok(None) => Span::new(0, 0),
            | Err(error) => {
                let mut span_error = self.span_error.borrow_mut();
                if span_error.is_none() {
                    *span_error = Some(error);
                }
                Span::new(0, 0)
            },
        }
    }

    pub fn take_span_error(&self) -> Option<ReadError> {
        self.span_error.borrow_mut().take()
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

    pub fn req_array<'v>(
        &self,
        node: &'v Value,
        field: &'static str,
    ) -> Result<&'v [Value], ReadError> {
        let value: &Value =
            node.get(field).ok_or_else(|| self.missing(node, field))?;

        let array: &[Value] = value
            .as_array()
            .ok_or_else(|| self.invalid(node, field, "an array"))?;

        Ok(array)
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

    pub fn text_source(
        &self,
        node: &Value,
        field: &'static str,
    ) -> Result<oxc::str::Str<'a>, ReadError> {
        let value_node: &Value =
            node.get(field).ok_or_else(|| self.missing(node, field))?;

        let value: &str = value_node
            .as_str()
            .ok_or_else(|| self.invalid(node, field, "a string"))?;

        Ok(json::read_source_str(value, &self.builder))
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

    pub fn property_key(
        &self,
        node: &Value,
    ) -> Result<PropertyKey<'a>, ReadError> {
        let computed: bool = self.flag(node, "computed");

        self.req(node, "key", |cx, key_node| {
            targets::read_property_key(cx, key_node, computed)
        })
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
