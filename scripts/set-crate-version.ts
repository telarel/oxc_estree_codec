import type { ConsolaInstance } from "consola";

import * as Fsp from "node:fs/promises";
import * as Path from "node:path";
import * as Process from "node:process";
import * as Url from "node:url";

import { parse, patch } from "@decimalturn/toml-patch";
import { createConsola } from "consola";

const __dirname: string = Path.dirname(Url.fileURLToPath(import.meta.url));

const workspaceRoot: string = Path.resolve(__dirname, "..");

const WORKSPACE_MANIFEST_PATH: string = Path.resolve(
    workspaceRoot,
    "Cargo.toml",
);

const CRATE_MANIFEST_PATH: string = Path.resolve(
    workspaceRoot,
    "crate",
    "Cargo.toml",
);

const consola: ConsolaInstance = createConsola({
    formatOptions: {
        date: false,
    },
});

type CrateManifest = {
    package: {
        name: string;
        version: string;
    };
};

type WorkspaceDependency = {
    version: string;
};

type WorkspaceManifest = {
    workspace: {
        dependencies: Record<string, WorkspaceDependency>;
    };
};

const readVersionArgument = (argv: ReadonlyArray<string>): string => {
    const value: string | undefined = argv[2];

    if (value === void 0 || value.length === 0) {
        throw new Error(
            "Usage: node ./scripts/bump-crate-versions.ts <version>",
        );
    }

    return value;
};

type WithUpdatedCrateVersionOptions = {
    manifest: CrateManifest;
    version: string;
};

const withUpdatedCrateVersion = (
    options: WithUpdatedCrateVersionOptions,
): CrateManifest => {
    return {
        ...options.manifest,
        package: {
            ...options.manifest.package,
            version: options.version,
        },
    };
};

type WithUpdatedDependencyVersionOptions = {
    manifest: WorkspaceManifest;
    crateName: string;
    version: string;
};

const withUpdatedDependencyVersion = (
    options: WithUpdatedDependencyVersionOptions,
): WorkspaceManifest => {
    const dependency: WorkspaceDependency | undefined =
        options.manifest.workspace.dependencies[options.crateName];

    if (dependency === void 0) {
        return options.manifest;
    }

    return {
        ...options.manifest,
        workspace: {
            ...options.manifest.workspace,
            dependencies: {
                ...options.manifest.workspace.dependencies,
                [options.crateName]: {
                    ...dependency,
                    version: options.version,
                },
            },
        },
    };
};

const version: string = readVersionArgument(Process.argv);

const crateSource: string = await Fsp.readFile(CRATE_MANIFEST_PATH, "utf-8");

const crateManifest: CrateManifest = parse(crateSource) as CrateManifest;

const crateName: string = crateManifest.package.name;

const previousVersion: string = crateManifest.package.version;

const workspaceSource: string = await Fsp.readFile(
    WORKSPACE_MANIFEST_PATH,
    "utf-8",
);

const workspaceManifest: WorkspaceManifest = parse(
    workspaceSource,
) as WorkspaceManifest;

const updatedCrateSource: string = patch(
    crateSource,
    withUpdatedCrateVersion({ manifest: crateManifest, version }),
);

const updatedWorkspaceSource: string = patch(
    workspaceSource,
    withUpdatedDependencyVersion({
        manifest: workspaceManifest,
        crateName,
        version,
    }),
);

if (
    updatedCrateSource === crateSource &&
    updatedWorkspaceSource === workspaceSource
) {
    consola.info(`Rust crate already at ${version}.`);
} else {
    await Fsp.writeFile(CRATE_MANIFEST_PATH, updatedCrateSource, "utf-8");

    await Fsp.writeFile(
        WORKSPACE_MANIFEST_PATH,
        updatedWorkspaceSource,
        "utf-8",
    );

    consola.success(
        `Bumped Rust crate ${crateName}: ${previousVersion} -> ${version}.`,
    );
}
