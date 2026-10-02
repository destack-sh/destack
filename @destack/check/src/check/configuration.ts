import { readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { isDeepStrictEqual } from "node:util";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as lint from "../lint/plugin.ts";
import type { Plugin } from "../lint/plugin.ts";
import { CheckError } from "../error/index.ts";

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
export function lintConfiguration(plugins: readonly Plugin[] = [], absolute = false) {
    // load the built-in rules by path for managed checks and by export for editors
    const builtin = absolute
        ? fileURLToPath(new URL("../lint/index.ts", import.meta.url))
        : "@destack/check/lint";

    // reserve each namespace before enabling the provider's complete rule set
    const names = new Set(["destack", "typescript", "oxc", "unicorn", "eslint"]);
    const rules: Record<string, "error"> = {};
    for (const plugin of plugins) {
        if (names.has(plugin.name)) {
            throw new CheckError("configuration", `duplicate lint plugin: ${plugin.name}`);
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

            // single-letter names outside indices and vector components
            "eslint/id-length": ["error", { min: 2, exceptions: ["i", "j", "x", "y", "z", "_"] }],

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
                // let the devtools exporter and the build worker write to the console
                files: [
                    "**/@destack/telemetry/src/browser/devtools.ts",
                    "**/@destack/build/src/build/worker.ts",
                ],
                rules: { "eslint/no-console": "off" },
            },
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
        ],
        options: { denyWarnings: true, typeAware: true, respectEslintDisableDirectives: false },
    };
}

/** Write editor and direct-tool configuration from the canonical settings. */
export async function configurePackage(
    directory: string,
    plugins: readonly Plugin[] = [],
): Promise<void> {
    // write equivalent portable settings for direct Oxc invocations
    await Promise.all([
        writeFile(
            resolve(directory, ".oxlintrc.json"),
            `${JSON.stringify(lintConfiguration(plugins), null, 4)}\n`,
        ),
        writeFile(
            resolve(directory, ".oxfmtrc.json"),
            `${JSON.stringify(formatConfiguration, null, 4)}\n`,
        ),
    ]);
}

/** Reject existing tool configuration that differs from the generated settings. */
export async function checkConfiguration(
    directory: string,
    plugins: readonly Plugin[] = [],
): Promise<void> {
    // permit omitted editor files while rejecting independent rule definitions
    const configurations = [
        [".oxlintrc.json", lintConfiguration(plugins)],
        [".oxfmtrc.json", formatConfiguration],
    ] as const;
    for (const [name, configuration] of configurations) {
        const path = resolve(directory, name);
        if (existsSync(path)) {
            const source = await readFile(path, "utf8");
            if (!isDeepStrictEqual(JSON.parse(source), configuration)) {
                throw new CheckError(
                    "configuration",
                    `${name} differs from the shared configuration; run configure`,
                );
            }
        }
    }
}
