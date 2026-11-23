import type { ParserPlugin } from "@babel/parser";

import * as Fs from "node:fs";
import * as Module from "node:module";
import * as Path from "node:path";
import * as Url from "node:url";

import { parse as babelParse } from "@babel/parser";
import { parse as typescriptEstreeParse } from "@typescript-eslint/typescript-estree";
import { parse as acornParse } from "acorn";
import { parse as espreeParse } from "espree";

type ParserMetadata = {
    readonly packageName: string;
    readonly options: string;
};

const PARSERS = {
    acorn: {
        packageName: "acorn",
        options:
            "ecmaVersion=latest, sourceType=module, ranges=true, locations=true",
    },
    babel: {
        packageName: "@babel/parser",
        options: "plugins=[estree], ranges=true",
    },
    espree: {
        packageName: "espree",
        options: "ecmaVersion=latest, sourceType=module, range=true, loc=true",
    },
    typescript_estree: {
        packageName: "@typescript-eslint/typescript-estree",
        options: "sourceType=module, range=true, loc=true",
    },
    esprima: {
        packageName: "esprima",
        options: "parseModule, range=true, loc=true, raw=true",
    },
} as const satisfies Record<string, ParserMetadata>;

type ParserName = keyof typeof PARSERS;

type SourceType = "module" | "script";

type Case = {
    readonly file: string;
    readonly parsers: readonly ParserName[];
    readonly sourceType?: SourceType;
};

type EsprimaOptions = {
    readonly range: boolean;
    readonly loc: boolean;
    readonly raw: boolean;
};

type EsprimaParser = {
    parseModule: (source: string, options: EsprimaOptions) => unknown;
    parseScript: (source: string, options: EsprimaOptions) => unknown;
};

type PackageJson = {
    readonly devDependencies?: Record<string, string>;
};

type ManifestParser = {
    readonly version: string;
    readonly options: string;
};

type ManifestFixture = {
    readonly parser: ParserName;
    readonly source: string;
    readonly json: string;
};

type Manifest = {
    readonly generatedBy: string;
    readonly parsers: Record<ParserName, ManifestParser>;
    readonly fixtures: readonly ManifestFixture[];
};

const PACKAGE_ROOT: string = Path.dirname(
    Path.dirname(Url.fileURLToPath(import.meta.url)),
);
const CASES_DIR: string = Path.join(PACKAGE_ROOT, "cases");
const FIXTURES_DIR: string = Path.join(PACKAGE_ROOT, "fixtures");

// Esprima 4 ships no type declarations and this package intentionally does not
// add `@types/esprima`, so load it through a typed require boundary.
const nodeRequire: NodeRequire = Module.createRequire(import.meta.url);
const esprima: EsprimaParser = nodeRequire("esprima") as EsprimaParser;

const PARSER_NAMES: readonly ParserName[] = Object.keys(
    PARSERS,
) as ParserName[];

const ALL_NO_ESPRIMA: readonly ParserName[] = PARSER_NAMES.filter(
    (parser: ParserName): boolean => parser !== "esprima",
);

const CASES: readonly Case[] = [
    { file: "empty.js", parsers: PARSER_NAMES },
    { file: "basic.js", parsers: PARSER_NAMES },
    { file: "modern.js", parsers: ALL_NO_ESPRIMA },
    { file: "templates.js", parsers: PARSER_NAMES },
    { file: "modern_templates.js", parsers: ALL_NO_ESPRIMA },
    { file: "unicode.js", parsers: PARSER_NAMES },
    { file: "literals.js", parsers: PARSER_NAMES },
    { file: "bigint.js", parsers: ALL_NO_ESPRIMA },
    { file: "comments.js", parsers: PARSER_NAMES },
    { file: "hashbang.js", parsers: ALL_NO_ESPRIMA },
    { file: "script.js", parsers: PARSER_NAMES, sourceType: "script" },
    {
        file: "jsx.jsx",
        parsers: ["espree", "babel", "typescript_estree"],
    },
    { file: "types.ts", parsers: ["typescript_estree"] },
    { file: "types.tsx", parsers: ["typescript_estree"] },
];

type ParseFixtureOptions = {
    readonly parser: ParserName;
    readonly file: string;
    readonly source: string;
    readonly sourceType: SourceType;
};

const parseFixture = (options: ParseFixtureOptions): unknown => {
    const jsx: boolean = options.file.endsWith("x");

    switch (options.parser) {
        case "acorn": {
            return acornParse(options.source, {
                ecmaVersion: "latest",
                sourceType: options.sourceType,
                locations: true,
                ranges: true,
            });
        }
        case "babel": {
            const plugins: ParserPlugin[] = jsx
                ? ["estree", "jsx"]
                : ["estree"];

            return babelParse(options.source, {
                sourceType: options.sourceType,
                plugins,
                ranges: true,
            }).program;
        }
        case "espree": {
            return espreeParse(options.source, {
                ecmaVersion: "latest",
                sourceType: options.sourceType,
                range: true,
                loc: true,
                comment: true,
                ecmaFeatures: { jsx },
            });
        }
        case "typescript_estree": {
            return typescriptEstreeParse(options.source, {
                sourceType: options.sourceType,
                range: true,
                loc: true,
                jsx,
            });
        }
        case "esprima": {
            const esprimaOptions: EsprimaOptions = {
                range: true,
                loc: true,
                raw: true,
            };

            return options.sourceType === "script"
                ? esprima.parseScript(options.source, esprimaOptions)
                : esprima.parseModule(options.source, esprimaOptions);
        }
    }

    throw new Error(`unsupported parser: ${String(options.parser)}`);
};

const serialize = (ast: unknown): string => {
    return JSON.stringify(ast, (_key: string, value: unknown): unknown =>
        typeof value === "bigint" ? null : value,
    );
};

const devDependencies = (): Record<string, string> => {
    const packageJson: PackageJson = JSON.parse(
        Fs.readFileSync(Path.join(PACKAGE_ROOT, "package.json"), "utf-8"),
    ) as PackageJson;

    return packageJson.devDependencies ?? {};
};

const throwMissingDevDependency = (packageName: string): never => {
    throw new Error(
        `missing devDependency ${packageName}; run pnpm add --save-dev ${packageName}`,
    );
};

const resolveParserVersions = (): Record<ParserName, ManifestParser> => {
    const dependencies: Record<string, string> = devDependencies();

    const versionOf = (packageName: string): string => {
        const version: string | undefined = dependencies[packageName];

        return typeof version === "string"
            ? version
            : throwMissingDevDependency(packageName);
    };

    return Object.fromEntries(
        PARSER_NAMES.map((parser: ParserName): [ParserName, ManifestParser] => [
            parser,
            {
                version: versionOf(PARSERS[parser].packageName),
                options: PARSERS[parser].options,
            },
        ]),
    ) as Record<ParserName, ManifestParser>;
};

const counts: Record<ParserName, number> = Object.fromEntries(
    PARSER_NAMES.map((parser: ParserName): [ParserName, number] => [parser, 0]),
) as Record<ParserName, number>;

const fixtures: ManifestFixture[] = [];

// The fixtures are generated, not tracked, so stale files from removed cases
// would otherwise keep being tested.
Fs.rmSync(FIXTURES_DIR, { recursive: true, force: true });

for (const testCase of CASES) {
    const source: string = Fs.readFileSync(
        Path.join(CASES_DIR, testCase.file),
        "utf-8",
    );
    const sourceType: SourceType = testCase.sourceType ?? "module";

    for (const parser of testCase.parsers) {
        const ast: unknown = parseFixture({
            parser,
            file: testCase.file,
            source,
            sourceType,
        });
        const json: string = `${parser}/${testCase.file}.json`;
        const outputPath: string = Path.join(FIXTURES_DIR, json);

        Fs.mkdirSync(Path.dirname(outputPath), { recursive: true });
        Fs.writeFileSync(outputPath, serialize(ast));

        fixtures.push({ parser, source: testCase.file, json });
        counts[parser] += 1;
    }
}

const manifest: Manifest = {
    generatedBy: "tests/estree/scripts/generate-estree-fixtures.ts",
    parsers: resolveParserVersions(),
    fixtures,
};

Fs.writeFileSync(
    Path.join(FIXTURES_DIR, "manifest.json"),
    `${JSON.stringify(manifest, null, 4)}\n`,
);

for (const parser of PARSER_NAMES) {
    console.log(`${parser}: ${counts[parser]} fixtures`);
}
