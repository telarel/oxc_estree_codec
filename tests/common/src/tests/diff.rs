use test_helpers::diff::{RoundtripError, roundtrip_bytes};

fn roundtrip_diff(
    file: &str,
    code: &str,
) {
    roundtrip_bytes(file, code).map(|_| ()).unwrap_or_else(
        |error| match error {
            | RoundtripError::ParseFailed => {
                panic!("fixture does not parse: {file}");
            },
            | RoundtripError::Roundtrip(message) => {
                panic!("{message}");
            },
        },
    );
}

#[test]
fn test_diff_plain_js_module() {
    roundtrip_diff(
        "m.mjs",
        "import def, { named } from 'mod';\nexport const e = 1;\nexport function fn(...rest) { return () => def(...rest); }\nexport default class extends named {}\n",
    );
}

#[test]
fn test_diff_class_heavy() {
    roundtrip_diff(
        "c.ts",
        "abstract class A extends Base implements I { #x = 1; static s: number = 2; declare d?: string; constructor(public readonly a: Foo, private b = 1) { super(); } get g(): number { return this.#x; } static { init(); } }\nclass B extends A<string> { @dec m<T>(x: T): asserts x is T {} }\nconst C = class Named<T extends object = {}> implements J<T> {};\n",
    );
}

#[test]
fn test_diff_jsx_tsx() {
    roundtrip_diff(
        "j.tsx",
        "const el = <Provider key={k} {...rest}><Item a: b>text{x}</Item></Provider>;\nconst frag = <>before{...spread}after</>;\nconst ns = <svg:rect x=\"1\" />;\n",
    );
}

#[test]
fn test_diff_ts_types() {
    roundtrip_diff(
        "t.ts",
        "type A = { a: string; b?: readonly number[] };\ntype B<T> = T extends infer U ? U : never;\ninterface I extends J<K> { m(x: this): void; }\nenum E { A = 1, B = A + 1 }\ndeclare function f<T>(x: T): x is T & {};\ntype M = { [K in keyof T as `get${K & string}`]+?: T[K] };\nconst u: unique symbol = Symbol();\n",
    );
}

#[test]
fn test_diff_literals_and_expressions() {
    roundtrip_diff(
        "l.ts",
        "const t = `head ${x} mid ${tag`raw`} tail`;\nconst re = /ab+c/giv;\nconst big = 0xFFn;\nconst nums = [0, 1.5, 1e10, 0o17, 0b101];\nconst str = \"\\u{1F600}\";\nvoid 0; typeof x; delete o.p; a ?? b; c?.d; e ??= f;\nlet { a = 1, ...rest } = obj;\nlabel: for (;;) { break label; }\n",
    );
}

#[test]
fn test_diff_directives_and_hashbang() {
    roundtrip_diff(
        "h.js",
        "#!/usr/bin/env node\n\"use strict\";\n'use other';\nfunction fn() { \"use asm\"; }\n;\n",
    );
}
