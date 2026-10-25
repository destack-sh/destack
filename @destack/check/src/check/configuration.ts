import type { FileSystem } from "@destack/environment";
import { isDeepStrictEqual } from "node:util";
import { dirname, relative, resolve, sep } from "node:path";
import { Definition, type PackageDefinition } from "@destack/package";
import { TYPESCRIPT_OPTIONS } from "@destack/package/build";
import * as lint from "../lint/plugin.ts";
import type { Plugin } from "../lint/plugin.ts";
import type { LintSettings } from "../lint/package.ts";
import { CheckError } from "../error/index.ts";
import { expectationOverrides, readExpectations, workspaceMembers } from "./expectation.ts";
import { type Manifest, readManifest } from "./manifest.ts";
import { formatSource } from "./format.ts";
import { present } from "@destack/schema";

/** A bunfig.toml preload list, its quoted modules captured. */
const PRELOAD = /^\s*preload\s*=\s*\[([^\]]*)\]/gmu;

/** A module a script runs directly, such as `scripts/refresh.ts`. */
const SCRIPT_MODULE = /^[\w./@-]+\.[cm]?[jt]sx?$/u;

/** The operators separating the commands of a package script. */
const SCRIPT_COMMANDS = /&&|\|\||;/u;

/** The directories a package's compiler configuration includes beside its configuration files. */
const SOURCES = ["src", "tests"] as const;

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
    settings: LintSettings,
    options: {
        /** The trusted plugins beside the built-in one. */
        readonly plugins?: readonly Plugin[];
        /** The module of the built-in rules, its export for editors by default. */
        readonly builtin?: string;
        /** The rules expectations turn off, by file. */
        readonly overrides?: readonly { files: string[]; rules: Record<string, "off"> }[];
    } = {},
) {
    // load the built-in rules by export for editors, or the module a managed check names
    const { plugins = [], builtin = "@destack/check/lint", overrides = [] } = options;

    // reserve each namespace before enabling the provider's complete rule set
    const names = new Set(["destack", "typescript", "oxc", "unicorn", "eslint"]);
    const rules: Record<string, "error"> = {};
    for (const plugin of plugins) {
        if (names.has(plugin.name)) {
            throw CheckError.of("CONFIGURATION_INVALID", `duplicate lint plugin: ${plugin.name}`);
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

            // type every value precisely: no assertions, non-null claims, any or directives
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
                // stop server code from importing declarations and client-safe layers
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
        settings: {
            destack: { packages: [...settings.packages], entrypoints: [...settings.entrypoints] },
        },
        options: { denyWarnings: true, typeAware: true, respectEslintDisableDirectives: false },
    };
}

/** A workspace member defining a Destack package: its directory and package manifest. */
interface WorkspaceMember {
    /** The member's directory. */
    readonly directory: string;
    /** The member's package.json. */
    readonly manifest: Manifest;
}

/** Read the lint settings of a settings root: its own package, or its workspace's packages. */
export async function lintSettings(directory: string, files: FileSystem): Promise<LintSettings> {
    // read the root and every workspace member defining a Destack package
    const members: WorkspaceMember[] = [];
    const declared = new Map<string, readonly string[]>();
    for (const member of [directory, ...(await workspaceMembers(directory, files))]) {
        const definition = await readDefinition(member, files);
        if (definition !== undefined) {
            members.push({
                directory: member,
                manifest: (await readManifest(member, files)) ?? {},
            });
            declared.set(member, definition.entrypoints ?? []);
        }
    }

    // take the modules each package's bin, declared entrypoints, Bun scripts and preloads run
    const entrypoints: string[] = [];
    for (const member of members) {
        const { bin = {}, scripts = {} } = member.manifest;
        const run = Object.values(scripts).flatMap((script) => scriptModules(script));
        const programs = typeof bin === "string" ? [bin] : Object.values(bin);
        const starts = declared.get(member.directory) ?? [];
        const preloads = await readPreloads(member.directory, members, files);
        for (const program of [...programs, ...starts, ...run]) {
            entrypoints.push(resolve(member.directory, program));
        }
        entrypoints.push(...preloads);
    }

    return {
        packages: members.map(
            (member) => relative(directory, member.directory).split(sep).join("/") || ".",
        ),
        entrypoints: [
            ...new Set(entrypoints.map((path) => relative(directory, path).split(sep).join("/"))),
        ],
    };
}

/** Read the modules a package script runs with Bun: each command's module after `bun run`. */
function scriptModules(script: string): string[] {
    return script.split(SCRIPT_COMMANDS).flatMap((command) => {
        // skip Bun's flags and its run subcommand, taking the module it runs
        const [program, ...rest] = command.trim().split(/\s+/u);
        const module = rest.find((word) => !word.startsWith("-") && word !== "run");

        return program === "bun" && module !== undefined && SCRIPT_MODULE.test(module)
            ? [module]
            : [];
    });
}

/** Read the modules a package's bunfig.toml preloads, each package export from its member. */
async function readPreloads(
    directory: string,
    members: readonly WorkspaceMember[],
    files: FileSystem,
): Promise<string[]> {
    // read the preload lists, absent without a bunfig.toml
    const path = resolve(directory, "bunfig.toml");
    const text = (await files.exists(path)) ? await files.readTextFile(path) : "";
    const specifiers = [...text.matchAll(PRELOAD)].flatMap((match) =>
        [...present(match[1], "a preload list").matchAll(/"([^"]+)"/gu)].map((quoted) =>
            present(quoted[1], "a quoted preload"),
        ),
    );

    // resolve each specifier to the module it loads
    return specifiers.flatMap((specifier) => {
        const member = members.find(({ manifest: { name } }) =>
            name === undefined ? false : specifier === name || specifier.startsWith(`${name}/`),
        );

        // resolve a relative module against the package
        if (specifier.startsWith(".")) {
            return [resolve(directory, specifier)];
        }
        // leave a dependency's preload, whose module lies outside the workspace
        else if (member === undefined) {
            return [];
        }
        // resolve a member's export to its one module, refusing an export of conditions
        else {
            const name = present(member.manifest.name, "the member's name");
            const subpath = specifier === name ? "." : `.${specifier.slice(name.length)}`;
            const target = member.manifest.exports?.[subpath];
            if (typeof target !== "string") {
                throw CheckError.of(
                    "CONFIGURATION_INVALID",
                    `${specifier} is no single module its package exports`,
                );
            }

            return [resolve(member.directory, target)];
        }
    });
}

/** Produce a package's compiler configuration from its manifests and sources. */
export async function typescriptConfiguration(directory: string, files: FileSystem) {
    // read the package's name and declared dependency names
    const manifest = (await readManifest(directory, files)) ?? {};
    const names = new Set([
        ...Object.keys(manifest.dependencies ?? {}),
        ...Object.keys(manifest.devDependencies ?? {}),
    ]);
    const uses = (name: string) => manifest.name === name || names.has(name);

    // read the runtimes of the package and of each export
    const definition = await readDefinition(directory, files);
    const runtimes = new Set([
        ...(definition?.runtimes ?? []),
        ...Object.values(definition?.exports ?? {}).flatMap((entry) => entry.runtimes),
    ]);

    // separate nested packages and find the views among the package's sources
    const sources = await readSources(directory, files);
    const hasViews = uses("@destack/view") && sources.hasViews;

    // select ambient types from the declared type packages, the view library and the build
    const isBun = names.has("@types/bun") || names.has("bun-types");
    const types = [
        ...runtimeTypes(names),
        ...(hasViews && names.has("@destack/view") ? ["@destack/view/client"] : []),
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

/** Select the JSX compiler options: the terminal renderer's, else the view library's. */
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
async function readSources(directory: string, files: FileSystem) {
    // walk the included directories, stopping at each directory with a manifest
    const packages: string[] = [];
    let hasViews = false;
    const pending: string[] = [];
    for (const name of SOURCES) {
        if (await files.exists(resolve(directory, name))) {
            pending.push(name);
        }
    }
    for (let path = pending.pop(); path !== undefined; path = pending.pop()) {
        for (const entry of await files.readDirectory(resolve(directory, path))) {
            const child = `${path}/${entry.name}`;
            if (entry.kind === "directory" && entry.name !== "node_modules") {
                if (await files.exists(resolve(directory, child, "package.json"))) {
                    packages.push(child);
                } else {
                    pending.push(child);
                }
            } else if (entry.kind === "file" && entry.name.endsWith(".tsx")) {
                hasViews = true;
            }
        }
    }

    return { hasViews, packages: packages.toSorted() };
}

/** Write editor and direct-tool configuration from the canonical settings. */
export async function configurePackage(
    directory: string,
    files: FileSystem,
    plugins: readonly Plugin[] = [],
): Promise<void> {
    // write the compiler settings of a Destack package or workspace root
    if (await files.exists(resolve(directory, "destack.json"))) {
        await writeJson(
            resolve(directory, "tsconfig.json"),
            await typescriptConfiguration(directory, files),
            files,
        );
    }

    // write the lint and format settings once per workspace, which Oxc finds above each file
    if (await hasToolSettings(directory, files)) {
        await Promise.all([
            writeJson(
                resolve(directory, ".oxlintrc.json"),
                await editorLintConfiguration(directory, files, plugins),
                files,
            ),
            writeJson(resolve(directory, ".oxfmtrc.json"), formatConfiguration, files),
        ]);
    }
}

/** Produce the lint configuration editors read at a settings root. */
async function editorLintConfiguration(
    directory: string,
    files: FileSystem,
    plugins: readonly Plugin[],
) {
    const overrides = expectationOverrides(await readExpectations(directory, files), directory);

    return lintConfiguration(await lintSettings(directory, files), { plugins, overrides });
}

/** Write a configuration as formatted JSON, as the formatter writes package files. */
async function writeJson(path: string, value: unknown, files: FileSystem): Promise<void> {
    await files.writeTextFile(path, await formatSource(path, `${JSON.stringify(value)}\n`));
}

/** Reject existing tool configuration that differs from the generated settings. */
export async function checkConfiguration(
    directory: string,
    files: FileSystem,
    plugins: readonly Plugin[] = [],
): Promise<void> {
    // build each configuration a file exists for, with compiler settings for Destack packages alone
    const configurations = [
        [".oxlintrc.json", () => editorLintConfiguration(directory, files, plugins)],
        [".oxfmtrc.json", () => formatConfiguration],
        [
            "tsconfig.json",
            async () =>
                (await files.exists(resolve(directory, "destack.json")))
                    ? await typescriptConfiguration(directory, files)
                    : undefined,
        ],
    ] as const;
    for (const [name, build] of configurations) {
        // refuse an existing file that differs from the generated configuration
        const path = resolve(directory, name);
        const configuration = (await files.exists(path)) ? await build() : undefined;
        if (configuration !== undefined) {
            const source = await files.readTextFile(path);
            if (!isDeepStrictEqual(JSON.parse(source), configuration)) {
                throw CheckError.of(
                    "CONFIGURATION_INVALID",
                    `${name} differs from the shared configuration; run configure`,
                );
            }
        }
    }
}

/** Decide whether a directory has the lint and format settings. */
async function hasToolSettings(directory: string, files: FileSystem): Promise<boolean> {
    // put them at a workspace root and in a template
    if ((await readManifest(directory, files))?.workspaces !== undefined) {
        return true;
    }
    const definition = await readDefinition(directory, files);
    if (definition?.template !== undefined) {
        return true;
    }

    // put them in a package that no enclosing workspace covers
    for (
        let parent = dirname(resolve(directory));
        parent !== dirname(parent);
        parent = dirname(parent)
    ) {
        if ((await readManifest(parent, files))?.workspaces !== undefined) {
            return false;
        }
    }

    return true;
}

/** Find the directory whose tool settings cover a directory: itself or the nearest one above. */
export async function settingsRoot(directory: string, files: FileSystem): Promise<string> {
    let current = resolve(directory);
    while (!(await hasToolSettings(current, files)) && current !== dirname(current)) {
        current = dirname(current);
    }

    return current;
}

/** Read the package a directory's destack.json defines. */
async function readDefinition(
    directory: string,
    files: FileSystem,
): Promise<PackageDefinition | undefined> {
    const path = resolve(directory, "destack.json");

    return (await files.exists(path))
        ? Definition.package(Definition.read(await files.readTextFile(path)))
        : undefined;
}
