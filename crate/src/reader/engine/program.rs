use oxc::allocator::{Allocator, Vec as ArenaVec};
use oxc::ast::ast::{
    Comment, Directive, FunctionBody, Hashbang, Program, SourceType, Statement,
    StringLiteral,
};
use oxc::ast::builder::AstBuilder;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use crate::errors::read::ReadError;
use crate::reader::engine::context::{Cx, Seg, ty_of};
use crate::reader::json::Value;
use crate::reader::literal;
use crate::reader::statement;

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

    let directive_str: &str = node
        .get("directive")
        .and_then(Value::as_str)
        .ok_or_else(|| cx.invalid(node, "directive", "a string"))?;

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
    let value_node: &Value =
        node.get("value").ok_or_else(|| cx.missing(node, "value"))?;

    let value: &str = value_node
        .as_str()
        .ok_or_else(|| cx.invalid(node, "value", "a string"))?;

    let value_atom: oxc::str::Str<'a> =
        oxc::str::Str::from_str_in(value, cx.builder());

    let hashbang: Hashbang<'a> =
        Hashbang::new(cx.span(node), value_atom, cx.builder());

    Ok(hashbang)
}

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
