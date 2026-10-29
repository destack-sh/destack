import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { present } from "@destack/schema";
import solid, { type StartOptions } from "@solidjs/vite-plugin";
import { parseSync, Visitor } from "rolldown/utils";
import type { Plugin, PluginOption } from "@destack/package/build";
import { ViewError } from "../error/error.ts";

/** The package declaring views, whose installation resolves the framework's generated imports. */
export const VIEW_PACKAGE = "@destack/view";

/** The module the compiled JSX of every module imports, bundled dependencies' included. */
const RUNTIME_MODULE = `${VIEW_PACKAGE}/runtime`;

/** Return the Solid and framework plugins, in tests too. */
export function viewPlugins(ssr: boolean, start?: StartOptions): PluginOption[] {
    return [
        frameworkPlugin(),
        solid({
            ssr,
            serverFunctions: false,
            solid: { moduleName: RUNTIME_MODULE, hydratable: ssr },
            ...(start === undefined ? {} : { start: { ...start, env: start.env ?? false } }),
        }),
    ];
}

/** Refuse Solid server functions and resolve the framework's and the runtime's imports. */
function frameworkPlugin(): Plugin {
    // start resolutions at this installation, or at the project's package.json once configured
    const importer = fileURLToPath(import.meta.url);
    let isDevelopment = false;
    let project: string | undefined;

    return {
        name: `${VIEW_PACKAGE}/framework`,
        enforce: "pre",
        options(options) {
            // take the project from the bundler's working directory, which Vite's root replaces below
            if (options.cwd !== undefined) {
                project = join(options.cwd, "package.json");
            }
        },
        configResolved(configuration) {
            isDevelopment = configuration.command === "serve";
            project = join(configuration.root, "package.json");
        },
        transform: {
            filter: { id: /\.[cm]?[jt]sx?(?:\?|$)/u, code: /["']use server["']/u },
            handler(code, id) {
                // reject server-function directives before framework compilation
                const [path = id] = id.split("?");
                const parsed = parseSync(path, code);
                if (parsed.errors.length) {
                    throw ViewError.of("SOURCE_INVALID", `invalid framework source: ${id}`);
                }
                new Visitor({
                    ExpressionStatement(node) {
                        if (node.directive === "use server") {
                            throw ViewError.of(
                                "SERVER_FUNCTION_UNSUPPORTED",
                                `solid server functions are unsupported: ${id}:${node.start}`,
                            );
                        }
                    },
                }).visit(parsed.program);
            },
        },
        resolveId: {
            filter: { id: /^(?:solid-js|@solidjs\/web|@destack\/view\/runtime)(?:\/|$)/u },
            async handler(source, origin) {
                // tell runtime imports from generated framework imports
                const isRuntime =
                    source === RUNTIME_MODULE || source.startsWith(`${RUNTIME_MODULE}/`);
                const isGenerated =
                    origin?.replace(/^\0/u, "").startsWith("virtual:solid-") === true;

                // resolve the runtime as the project's dependency, whichever module imports it
                if (isRuntime) {
                    return await this.resolve(
                        source,
                        present(project, "the project's package.json"),
                        {
                            skipSelf: true,
                        },
                    );
                }
                // resolve generated imports and development refresh modules from this installation
                else if (isDevelopment || isGenerated) {
                    return await this.resolve(source, importer, { skipSelf: true });
                }
                // leave every other framework import to the project's resolution
                else {
                    return null;
                }
            },
        },
    };
}
