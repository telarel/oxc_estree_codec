use oxc::allocator::Allocator;
use oxc::span::SourceType;
use sonic_rs::Value;

use oxc_estree_codec::__internal::ProgramReader;

pub fn parse_value(json: &str) -> Value {
    sonic_rs::from_str::<Value>(json).unwrap()
}

fn roundtrip(
    file: &str,
    code: &str,
) -> String {
    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(
            &allocator,
            code,
            SourceType::from_path(file).unwrap_or_default(),
        )
        .parse();

    assert!(parser_return.diagnostics.is_empty(), "fixture must parse: {file}");

    let json: String = parser_return.program.to_estree_json(true, false);

    let value: Value = parse_value(&json);

    let reader: ProgramReader<'_> = ProgramReader::new(&allocator);

    let program: oxc::ast::ast::Program<'_> = reader
        .read(&value, parser_return.program.source_type, code)
        .expect("reader round-trips serializer output");

    oxc::codegen::Codegen::new().build(&program).code
}

#[test]
fn test_roundtrip_types() {
    let code: &str = "type M<T> = { [K in keyof T as `get${K & string}`]?: T[K] };\ninterface Ext extends A<number>, B { readonly [k: string]: unknown; m(x: string): void; }\nenum E { A = 1, B }\nnamespace NS { export const x = 1; }\ndeclare const dc: M<string>;\nfunction fn<T extends object = {}>(x: T): asserts x is T { throw new Error(); }";
    roundtrip("t.ts", code);
}

#[test]
fn test_roundtrip_jsx() {
    let code: &str = "const el = <div a={1} {...props}>text{x}</div>;\nconst frag = <>text</>;";
    roundtrip("j.tsx", code);
}

#[test]
fn test_roundtrip_literals() {
    let code: &str = "const t = `head ${x} mid ${y} tail`;\nconst re = /ab+c/gi;\nconst big = 123n;\nconst neg = -x;\nconst cond = x ? y : z;\nconst seq = (a, b, c);\ntag`t`;\nx++; --y;\ndelete x.z;\nvoid 0;\ntypeof x;\nconst fn2 = async (a) => a;\nnew Ctor(1);\na instanceof B;\na in b;";
    roundtrip("l.ts", code);
}

#[test]
fn test_roundtrip_variable_declaration() {
    let code: &str = "const greeting = \"hello\";";
    assert_eq!(roundtrip("a.ts", code), "const greeting = \"hello\";\n");
}

#[test]
fn test_roundtrip_function_with_default_and_rest() {
    let code: &str = "function fn(a, b = 1, ...rest) { return a + b + rest; }";
    assert_eq!(
        roundtrip("b.ts", code),
        "function fn(a, b = 1, ...rest) {\n\treturn a + b + rest;\n}\n"
    );
}

#[test]
fn test_roundtrip_statements() {
    let code: &str = "if (a) { b; } else c;\nfor (let i = 0; i < 9; i++) {}\nfor (const k in obj) {}\nfor (const v of arr) {}\nwhile (x) {}\ndo y(); while (z);\ntry { a; } catch ({ message }) { b; } finally { c; }\nswitch (v) { case 1: break; default: }\nlabel: debugger;\n;";
    roundtrip("s.ts", code);
}

#[test]
fn test_roundtrip_directive_and_hashbang() {
    let code: &str = "#!/usr/bin/env node\n\"use strict\";\n'use other';\n;";
    let out: String = roundtrip("h.js", code);
    assert!(out.contains("use strict"));
}

#[test]
fn test_roundtrip_modules_and_class() {
    let code: &str = "import def, { named as alias, type T1 } from 'mod' with { type: \"json\" };\nimport * as ns from 'n';\nimport 'side';\nimport source ph from 'p';\nexport const e = 1;\nexport { e as renamed };\nexport { q } from 'r' with { k: \"v\" };\nexport default e;\nexport * from 'x';\nexport * as ns2 from 's';\nclass C extends Base { #priv = 1; static s = 2; static { s(); } get g() { return 1; } set g(v) { x = v; } async m() { await x; } constructor() { super(); } [comp]() {} }";
    roundtrip("m.ts", code);
}

#[test]
fn test_roundtrip_throw_continue_with() {
    let code: &str = "throw err;\ncontinue outer;\nwith (o) { }\nfor (i = 0; i < 3; i++) {}\nfor (k in obj) {}\nfor (v of arr) {}\ntry { a; } catch (e) { } finally { }\nswitch (v) { default: continue; }\n";
    roundtrip("t.js", code);
}

#[test]
fn test_roundtrip_import_outputs() {
    let code: &str = "import \"side\";";
    assert_eq!(roundtrip("i.mjs", code), "import \"side\";\n");
    let code: &str = "import x from 'm';";
    assert_eq!(roundtrip("i.mjs", code), "import x from \"m\";\n");
}

#[test]
fn test_roundtrip_class_and_module() {
    let code: &str = "import def, { named as alias, type T1 } from 'mod';\nexport const e = 1;\nexport { e as renamed };\nexport default e;\nexport * from 'x';\nclass C extends Base { #priv = 1; static s = 2; static { s(); } get g() { return 1; } async m() { await x; } constructor() { super(); } }";
    roundtrip("m.ts", code);
}

#[test]
fn test_roundtrip_class_expressions() {
    let code: &str = "const C = class Named extends Base {};";
    let out: String = roundtrip("c.ts", code);
    assert_eq!(out, "const C = class Named extends Base {};\n");
    let code: &str = "const D = class { m() {} };";
    let out: String = roundtrip("c.ts", code);
    assert_eq!(out, "const D = class {\n\tm() {}\n};\n");
}

#[test]
fn test_roundtrip_export_default_class() {
    let code: &str = "export default class {};";
    let out: String = roundtrip("m.ts", code);
    assert_eq!(out, "export default class {}\n;\n");
    let code: &str = "export default class Named {};";
    let out: String = roundtrip("m.ts", code);
    assert_eq!(out, "export default class Named {}\n;\n");
    let code: &str = "export default function () {};";
    let out: String = roundtrip("m.ts", code);
    assert_eq!(out, "export default function() {}\n;\n");
}

#[test]
fn test_roundtrip_class_decorators() {
    let code: &str = "@dec class C { @x m() {} @y p = 1; }";
    let out: String = roundtrip("d.ts", code);
    assert_eq!(out, "@dec class C {\n\t@x m() {}\n\t@y p = 1;\n}\n");
}

#[test]
fn test_roundtrip_class_heritage_and_implements() {
    let code: &str = "class C extends Base<Foo> implements I, J<T>, A.B.C {}";
    let out: String = roundtrip("h.ts", code);
    assert_eq!(out, "class C extends Base<Foo> implements I, J<T>, A.B.C {}\n");
}

#[test]
fn test_roundtrip_constructor_parameter_properties() {
    let code: &str = "class K { constructor(private a: Foo, public b = 1, readonly c, protected d?) { } }";
    let out: String = roundtrip("pp.ts", code);
    assert_eq!(
        out,
        "class K {\n\tconstructor(private a: Foo, public b = 1, readonly c, protected d?) {}\n}\n"
    );
}

#[test]
fn test_roundtrip_class_member_modifiers() {
    let code: &str = "class C { readonly x: Foo; declare y: Bar; private static z?; override m() {} protected n: Baz; }";
    let out: String = roundtrip("mm.ts", code);
    assert_eq!(
        out,
        "class C {\n\treadonly x: Foo;\n\tdeclare y: Bar;\n\tprivate static z?;\n\toverride m() {}\n\tprotected n: Baz;\n}\n"
    );
}

#[test]
fn test_roundtrip_function_this_param() {
    let code: &str = "function f(this: Foo, a, ...rest) { return this; }";
    let out: String = roundtrip("t.ts", code);
    assert_eq!(out, "function f(this: Foo, a, ...rest) {\n\treturn this;\n}\n");
}

#[test]
fn test_roundtrip_generic_type_arguments() {
    let code: &str = "foo<T>(x);\n";
    assert_eq!(roundtrip("g.ts", code), "foo<T>(x);\n");
    let code: &str = "new C<T>();\n";
    assert_eq!(roundtrip("g.ts", code), "new C<T>();\n");
    let code: &str = "tag<T>`x`;\n";
    assert_eq!(roundtrip("g.ts", code), "tag<T>`x`;\n");
}

#[test]
fn test_roundtrip_import_phase() {
    let code: &str = "import.source(f)(x);\n";
    assert_eq!(roundtrip("p.ts", code), "import.source(f)(x);\n");
    let code: &str = "import.defer(f)(x);\n";
    assert_eq!(roundtrip("p.ts", code), "import.defer(f)(x);\n");
}

#[test]
fn test_roundtrip_catch_type_annotation() {
    let code: &str = "try { a; } catch (e: unknown) { b; }\n";
    assert_eq!(
        roundtrip("c.ts", code),
        "try {\n\ta;\n} catch (e: unknown) {\n\tb;\n}\n"
    );
}

#[test]
fn test_roundtrip_destructuring_assignment() {
    let code: &str = "[a, b] = c;\n";
    assert_eq!(roundtrip("d.ts", code), "[a, b] = c;\n");
    let code: &str = "({ a } = obj);\n";
    assert_eq!(roundtrip("d.ts", code), "({a} = obj);\n");
    let code: &str = "[a, ...rest] = c;\n";
    assert_eq!(roundtrip("d.ts", code), "[a, ...rest] = c;\n");
    let code: &str = "({ a: b = 1, ...rest } = obj);\n";
    assert_eq!(roundtrip("d.ts", code), "({a: b = 1, ...rest} = obj);\n");
    let code: &str = "for ([a, b] of arr) {}\n";
    assert_eq!(roundtrip("d.ts", code), "for ([a, b] of arr) {}\n");
}

#[test]
fn test_roundtrip_private_member_access() {
    let code: &str = "class C { #f = 1; m() { return this.#f; } }";
    let out: String = roundtrip("p.ts", code);
    assert!(out.contains("this.#f"), "private member must round-trip: {out}");
}
