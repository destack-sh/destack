import solid, { type StartOptions } from "@solidjs/vite-plugin";
import { unplugin } from "@stylexjs/unplugin";
import { type PluginOption, type Plugin } from "vite";
import { type PackageSource } from "../source/index.ts";
import { modulePlugin } from "@destack/package/transform/vite";
import { fileURLToPath } from "node:url";
import { parseSync, Visitor, type ESTree, transformSync } from "rolldown/utils";
import { BuildError } from "../error/index.ts";
import { isAbsolute, relative, resolve, sep } from "node:path";

/** Use the same JSX, styles, and package metadata transforms in development and builds. */
export function compilationPlugins(
    project: PackageSource,
    ssr: boolean,
    start?: StartOptions,
): PluginOption[] {
    return [
        frameworkPlugin(),
        unplugin.vite({
            importSources: ["@stylexjs/stylex", "@destack/style"],
            useCSSLayers: true,
            cssInjectionTarget: (path) => path.includes("entry-client"),
            unstable_moduleResolution: { rootDir: project.directory, type: "commonJS" },
        }),
        solid({
            ssr,
            serverFunctions: false,
            solid: { moduleName: "@destack/view/runtime" },
            start: start && { ...start, env: start.env ?? false },
        }),
        modulePlugin(),
    ];
}

/** Resolve framework-generated imports against the build tool's declared dependencies. */
export function frameworkPlugin(): Plugin {
    const importer = fileURLToPath(import.meta.url);
    let isDevelopment = false;

    return {
        name: "destack-framework",
        enforce: "pre",
        configResolved(configuration) {
            isDevelopment = configuration.command === "serve";
        },
        transform: {
            filter: { id: /\.[cm]?[jt]sx?(?:\?|$)/, code: /["']use server["']/ },
            handler(code, id) {
                // reject server-function directives before framework compilation
                const parsed = parseSync(id.split("?")[0], code);
                if (parsed.errors.length) {
                    throw new BuildError("BUILD_FAILED", `Invalid framework source: ${id}`);
                }
                new Visitor({
                    ExpressionStatement(node) {
                        if (node.directive === "use server") {
                            throw new BuildError(
                                "BUILD_FAILED",
                                `Solid server functions are unsupported: ${id}:${node.start}`,
                            );
                        }
                    },
                }).visit(parsed.program);
            },
        },
        resolveId: {
            filter: { id: /^(?:solid-js|@solidjs\/web)(?:\/|$)/ },
            async handler(source, origin) {
                // resolve generated imports and development refresh modules from the compiler installation
                if (!isDevelopment && !origin?.replace(/^\0/, "").startsWith("virtual:solid-")) {
                    return;
                }
                return await this.resolve(source, importer, { skipSelf: true });
            },
        },
    };
}

/** Package-relative imports inserted into generated framework modules. */
const PREFIX = "virtual:destack-source/";

/** Give generated framework source stable imports and matching source maps. */
export function virtualPlugin(directory: string): Plugin {
    return {
        name: "destack-virtual-source",
        enforce: "pre",
        resolveId: {
            filter: { id: /^virtual:destack-source\// },
            async handler(source) {
                return await this.resolve(
                    resolve(directory, source.slice(PREFIX.length)),
                    undefined,
                    {
                        skipSelf: true,
                    },
                );
            },
        },
        transform: {
            filter: { id: /^\0?virtual:/ },
            handler(code, id) {
                // parse the generated module
                let isChanged = false;
                const parsed = parseSync(id, code);
                if (parsed.errors.length) {
                    throw new BuildError("BUILD_FAILED", `Invalid generated module: ${id}`);
                }

                // replace only parsed module specifiers within the application directory
                const imports = new Map<number, ESTree.StringLiteral>();
                const isManifest = id.replace(/^\0/, "") === "virtual:solid-manifest";
                new Visitor({
                    Literal(node) {
                        // normalize Vite's cwd-relative virtual entry names in the runtime manifest
                        if (
                            isManifest &&
                            typeof node.value === "string" &&
                            /(^|\/)virtual:solid-/.test(node.value)
                        ) {
                            imports.set(node.start, node as ESTree.StringLiteral);
                        }
                    },
                    ImportDeclaration(node) {
                        imports.set(node.source.start, node.source);
                    },
                    ExportNamedDeclaration(node) {
                        if (node.source) {
                            imports.set(node.source.start, node.source);
                        }
                    },
                    ExportAllDeclaration(node) {
                        imports.set(node.source.start, node.source);
                    },
                    ImportExpression(node) {
                        if (
                            node.source.type === "Literal" &&
                            typeof node.source.value === "string"
                        ) {
                            imports.set(node.source.start, node.source);
                        }
                    },
                }).visit(parsed.program);
                for (const imported of [...imports.values()].sort(
                    (left, right) => right.start - left.start,
                )) {
                    if (isManifest && /(^|\/)virtual:solid-/.test(imported.value)) {
                        const name = imported.value.slice(imported.value.indexOf("virtual:"));
                        code =
                            code.slice(0, imported.start) +
                            JSON.stringify(name) +
                            code.slice(imported.end);
                        isChanged = true;
                        continue;
                    }
                    if (!isAbsolute(imported.value)) {
                        continue;
                    }
                    const path = relative(directory, imported.value).split(sep).join("/");
                    if (path === ".." || path.startsWith("../") || isAbsolute(path)) {
                        continue;
                    }
                    code =
                        code.slice(0, imported.start) +
                        JSON.stringify(PREFIX + path) +
                        code.slice(imported.end);
                    isChanged = true;
                }
                if (!isChanged) {
                    return;
                }

                // start maps at the normalized generated source, preserving authored source maps
                const source = `virtual:destack/${id.slice(id.indexOf("virtual:") + 8)}`;
                const result = transformSync(source, code, {
                    jsx: "preserve",
                    sourcemap: true,
                    target: "esnext",
                });
                if (result.errors.length) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `Cannot transform generated module: ${id}`,
                    );
                }

                return { code: result.code, map: result.map };
            },
        },
    };
}
