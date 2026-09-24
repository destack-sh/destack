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
        categories: { correctness: "error" },
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
                    varsIgnorePattern: "^_",
                    caughtErrorsIgnorePattern: "^_",
                },
            ],

            // solid assigns JSX references during compilation
            "eslint/no-unassigned-vars": "off",

            // database queries implement PromiseLike and validators match control characters
            "unicorn/no-thenable": "off",
            "eslint/no-control-regex": "off",

            // callbacks and serialized class fields do not require a receiver or prototype
            "typescript/unbound-method": "off",
            "typescript/no-misused-spread": "off",

            // single-letter names outside indices and vector components
            "eslint/id-length": ["error", { min: 2, exceptions: ["i", "j", "x", "y", "z", "_"] }],

            // lowercase single-word or kebab-case file names
            "unicorn/filename-case": ["error", { case: "kebabCase" }],

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
