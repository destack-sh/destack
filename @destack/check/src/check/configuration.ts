import { readdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { isDeepStrictEqual } from "node:util";
import { dirname, resolve } from "node:path";
import { Definition, type PackageDefinition } from "@destack/package";
import { TYPESCRIPT_OPTIONS } from "@destack/package/build";
import * as lint from "../lint/plugin.ts";
import type { Plugin } from "../lint/plugin.ts";
import { CheckError } from "../error/index.ts";
import { expectationOverrides, readExpectations } from "./expectation.ts";
import { readManifest } from "./manifest.ts";
import { Toolchain } from "../toolchain/index.ts";
import { formatSource } from "./format.ts";

/** Fixed source formatting shared by editors and managed commands. */
export const formatConfiguration = {
    tabWidth: 4,
    printWidth: 100,
    useTabs: false,
    semi: true,
    singleQuote: false,
    trailingComma: "all",
    endOfLine: "lf",
    proseWrap: "preserve",
    sortImports: false,
    sortPackageJson: false,
} as const;

/** Produce the fixed rules and explicitly selected trusted plugins. */
export function lintConfiguration(
    plugins: readonly Plugin[] = [],
    absolute = false,
    overrides: readonly { files: string[]; rules: Record<string, "off"> }[] = [],
) {
    // load the built-in rules by path for managed checks and by export for editors
    const builtin = absolute ? Toolchain.lint() : "@destack/check/lint";

    // reserve each namespace before enabling the provider's complete rule set
    const names = new Set(["destack", "typescript", "oxc", "unicorn", "eslint"]);
    const rules: Record<string, "error"> = {};
    for (const plugin of plugins) {
        if (names.has(plugin.name)) {
            throw new CheckError("CONFIGURATION", `duplicate lint plugin: ${plugin.name}`);
        }
        names.add(plugin.name);
        for (const name of Object.keys(plugin.rules)) {
            rules[`${plugin.name}/${name}`] = "error";
        }
    }

    return {
        plugins: ["typescript", "oxc", "unicorn"],
        categories: { correctness: "error", suspicious: "error", perf: "error" },
        jsPlugins: [builtin, ...plugins.map(({ name, specifier }) => ({ name, specifier }))],
        rules: {
            "eslint/curly": ["error", "all"],
            "typescript/parameter-properties": ["error", { prefer: "class-property" }],
            "typescript/triple-slash-reference": [
                "error",
                { path: "always", types: "prefer-import", lib: "never" },
            ],
            "oxc/no-accumulating-spread": "error",
            "eslint/no-unused-vars": [
                "error",
                {
                    ignoreUsingDeclarations: true,
                    argsIgnorePattern: "^_",
                    ignoreRestSiblings: true,
                },
            ],

            // type every value precisely: no assertions, non-null claims, any, or compiler directives
            "typescript/consistent-type-assertions": ["error", { assertionStyle: "never" }],
            "typescript/no-non-null-assertion": "error",
            "typescript/no-explicit-any": "error",
            "typescript/ban-ts-comment": [
                "error",
                { "ts-expect-error": true, "ts-ignore": true, "ts-nocheck": true },
            ],
            "typescript/no-unsafe-type-assertion": "error",
            "typescript/no-unsafe-argument": "error",
            "typescript/no-unsafe-assignment": "error",
            "typescript/no-unsafe-call": "error",
            "typescript/no-unsafe-member-access": "error",
            "typescript/no-unsafe-return": "error",
            "typescript/no-floating-promises": "error",
            "typescript/no-misused-promises": "error",
            "typescript/strict-boolean-expressions": "error",
            "typescript/switch-exhaustiveness-check": "error",
            "typescript/unbound-method": "error",
            "typescript/no-misused-spread": "error",
            "typescript/only-throw-error": "error",
            "typescript/no-deprecated": "error",
            "typescript/no-unnecessary-type-assertion": "error",
            "eslint/preserve-caught-error": "error",
            "eslint/no-shadow": "error",
            "eslint/no-loop-func": "error",
            "eslint/no-promise-executor-return": "error",
            "eslint/require-unicode-regexp": "error",
            "unicorn/no-array-sort": "error",

            // solid assigns JSX references during compilation
            "eslint/no-unassigned-vars": "off",

            // database queries implement PromiseLike and validators match control characters
            "unicorn/no-thenable": "off",
            "eslint/no-control-regex": "off",

            // single-letter names outside indices, vector components and the message tag
            "eslint/id-length": [
                "error",
                { min: 2, exceptions: ["i", "j", "x", "y", "z", "_", "t"] },
            ],

            // lowercase single-word or kebab-case file names
            "unicorn/filename-case": ["error", { case: "kebabCase" }],

            // log through telemetry and print through a program's output, never the console
            "eslint/no-console": "error",

            // post to BroadcastChannel and workers without a target origin
            "unicorn/require-post-message-target-origin": "off",
            // type-only imports of ambient augmentations name nothing
            "unicorn/require-module-specifiers": "off",
            // build records as new values
            "oxc/no-map-spread": "off",
            // statements of one transaction run in order on one connection
            "eslint/no-await-in-loop": "off",
            // the compiler checks missing returns with types, through noImplicitReturns
            "typescript/consistent-return": "off",
            // a schema and its inferred type share one name
            "eslint/no-redeclare": "off",
            // validators read Zod's documented introspection API, `_zod`
            "eslint/no-underscore-dangle": "off",

            ...Object.fromEntries(
                Object.keys(lint.rules).map((name) => [`destack/${name}`, "error"]),
            ),
            ...rules,
        },
        overrides: [
            {
                // keep declarations and client-safe layers free of server code
                files: [
                    "**/src/service/**",
                    "**/src/connection/**",
                    "**/src/declare/**",
                    "**/src/stack/**",
                    "**/src/package.ts",
                ],
                rules: {
                    "eslint/no-restricted-imports": [
                        "error",
                        { patterns: ["**/server", "**/server/**", "*/server"] },
                    ],
                },
            },
            // accept the findings packages expect, so editors show what checking reports
            ...overrides,
        ],
        options: { denyWarnings: true, typeAware: true, respectEslintDisableDirectives: false },
    };
}

/** The directories a package's compiler configuration includes beside its configuration files. */
const SOURCES = ["src", "tests"] as const;

/** Produce a package's compiler configuration from its manifests and sources. */
export async function typescriptConfiguration(directory: string) {
    // read the package's name and declared dependency names
    const manifest = (await readManifest(directory)) ?? {};
    const names = new Set([
        ...Object.keys(manifest.dependencies ?? {}),
        ...Object.keys(manifest.devDependencies ?? {}),
    ]);
    const uses = (name: string) => manifest.name === name || names.has(name);

    // read the runtimes of the package and of each export
    const definition = await readDefinition(directory);
    const runtimes = new Set([
        ...(definition?.runtimes ?? []),
        ...Object.values(definition?.exports ?? {}).flatMap((entry) => entry.runtimes),
    ]);

    // separate nested packages and find the views among the package's sources
    const sources = await readSources(directory);
    const hasViews = uses("@destack/view") && sources.hasViews;

    // select ambient types from the declared type packages and the build's browser modules
    const isBun = names.has("@types/bun") || names.has("bun-types");
    const types = [
        ...runtimeTypes(names),
        ...(hasViews && names.has("@destack/web") ? ["@destack/web/client"] : []),
        ...(runtimes.has("browser") && names.has("@destack/build")
            ? ["@destack/build/browser"]
            : []),
        ...(names.has("vite") ? ["vite/client"] : []),
    ];

    // compile JSX for the view library or the terminal renderer
    const jsx = jsxOptions(names, hasViews);

    // declare web globals through DOM unless Bun's types declare them for server-only code
    const isDom =
        runtimes.has("browser") ||
        hasViews ||
        uses("@destack/style") ||
        uses("@opentui/solid") ||
        !isBun;

    return {
        compilerOptions: {
            ...TYPESCRIPT_OPTIONS,
            lib: isDom ? ["ESNext", "DOM", "DOM.Iterable"] : ["ESNext"],
            ...(types.length > 0 ? { types } : {}),
            ...jsx,
        },
        include: [...SOURCES, "*.config.ts"],
        ...(sources.packages.length > 0 ? { exclude: sources.packages } : {}),
    };
}

/** Select the runtime's ambient types: Bun's, else Node's, else none. */
function runtimeTypes(names: ReadonlySet<string>): string[] {
    // declare Bun's types
    if (names.has("@types/bun") || names.has("bun-types")) {
        return ["bun"];
    }
    // declare Node's types
    else if (names.has("@types/node")) {
        return ["node"];
    }
    // declare no runtime types
    else {
        return [];
    }
}

/** Select the JSX compiler options: the terminal renderer's, else the view library's for a package with views. */
function jsxOptions(
    names: ReadonlySet<string>,
    hasViews: boolean,
): { readonly jsx?: string; readonly jsxImportSource?: string } {
    // compile JSX for the terminal renderer
    if (names.has("@opentui/solid")) {
        return { jsx: "preserve", jsxImportSource: "@opentui/solid" };
    }
    // compile JSX for the view library
    else if (hasViews) {
        return { jsx: "preserve", jsxImportSource: "@destack/view" };
    }
    // compile no JSX
    else {
        return {};
    }
}

/** Find the nested packages and TSX modules in a package's included directories. */
async function readSources(directory: string) {
    // walk the included directories, stopping at each directory with its own manifest
    const packages: string[] = [];
    let hasViews = false;
    const pending: string[] = SOURCES.filter((name) => existsSync(resolve(directory, name)));
    for (let path = pending.pop(); path !== undefined; path = pending.pop()) {
        for (const entry of await readdir(resolve(directory, path), { withFileTypes: true })) {
            const child = `${path}/${entry.name}`;
            if (entry.isDirectory() && entry.name !== "node_modules") {
                if (existsSync(resolve(directory, child, "package.json"))) {
                    packages.push(child);
                } else {
                    pending.push(child);
                }
            } else if (entry.isFile() && entry.name.endsWith(".tsx")) {
                hasViews = true;
            }
        }
    }

    return { hasViews, packages: packages.toSorted() };
}

/** Write editor and direct-tool configuration from the canonical settings. */
export async function configurePackage(
    directory: string,
    plugins: readonly Plugin[] = [],
): Promise<void> {
    // write the compiler settings of a Destack package or workspace root
    if (existsSync(resolve(directory, "destack.json"))) {
        await writeJson(
            resolve(directory, "tsconfig.json"),
            await typescriptConfiguration(directory),
        );
    }

    // write the lint and format settings once per workspace, which Oxc finds above each file
    if (await holdsToolSettings(directory)) {
        await Promise.all([
            writeJson(
                resolve(directory, ".oxlintrc.json"),
                lintConfiguration(
                    plugins,
                    false,
                    expectationOverrides(await readExpectations(directory), directory),
                ),
            ),
            writeJson(resolve(directory, ".oxfmtrc.json"), formatConfiguration),
        ]);
    }
}

/** Write a configuration as formatted JSON, as the formatter keeps package files. */
async function writeJson(path: string, value: unknown): Promise<void> {
    await writeFile(path, await formatSource(path, `${JSON.stringify(value)}\n`));
}

/** Reject existing tool configuration that differs from the generated settings. */
export async function checkConfiguration(
    directory: string,
    plugins: readonly Plugin[] = [],
): Promise<void> {
    // permit omitted editor files while rejecting independent rule definitions, compiler settings only for Destack packages
    const configurations = [
        [
            ".oxlintrc.json",
            lintConfiguration(
                plugins,
                false,
                expectationOverrides(await readExpectations(directory), directory),
            ),
        ],
        [".oxfmtrc.json", formatConfiguration],
        ...(existsSync(resolve(directory, "destack.json"))
            ? [["tsconfig.json", await typescriptConfiguration(directory)] as const]
            : []),
    ] as const;
    for (const [name, configuration] of configurations) {
        const path = resolve(directory, name);
        if (existsSync(path)) {
            const source = await readFile(path, "utf8");
            if (!isDeepStrictEqual(JSON.parse(source), configuration)) {
                throw new CheckError(
                    "CONFIGURATION",
                    `${name} differs from the shared configuration; run configure`,
                );
            }
        }
    }
}

/** Decide whether a directory keeps the lint and format settings: a workspace root, a template copied out as its own project, or a package in no workspace. */
async function holdsToolSettings(directory: string): Promise<boolean> {
    // keep them at a workspace root and in a template
    if ((await readManifest(directory))?.workspaces !== undefined) {
        return true;
    }
    const definition = await readDefinition(directory);
    if (definition?.template !== undefined) {
        return true;
    }

    // keep them in a package that no enclosing workspace covers
    for (
        let parent = dirname(resolve(directory));
        parent !== dirname(parent);
        parent = dirname(parent)
    ) {
        if ((await readManifest(parent))?.workspaces !== undefined) {
            return false;
        }
    }

    return true;
}

/** Find the directory holding the tool settings that cover a directory: itself or the nearest one above. */
export async function settingsRoot(directory: string): Promise<string> {
    let current = resolve(directory);
    while (!(await holdsToolSettings(current)) && current !== dirname(current)) {
        current = dirname(current);
    }

    return current;
}

/** Read the package a directory's destack.json defines, absent without one or for a workspace root that is no package. */
async function readDefinition(directory: string): Promise<PackageDefinition | undefined> {
    const path = resolve(directory, "destack.json");

    return existsSync(path)
        ? Definition.package(Definition.read(await readFile(path, "utf8")))
        : undefined;
}
