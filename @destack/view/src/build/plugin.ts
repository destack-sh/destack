import { fileURLToPath } from "node:url";
import solid, { type StartOptions } from "@solidjs/vite-plugin";
import { parseSync, Visitor } from "rolldown/utils";
import type { Plugin, PluginOption } from "@destack/package/build";

/** The package declaring views, whose installation resolves the framework's generated imports. */
export const VIEW_PACKAGE = "@destack/view";

/** Return the Solid and framework plugins, rendering on the server when asked. */
export function viewPlugins(ssr: boolean, start?: StartOptions): PluginOption[] {
    return [
        frameworkPlugin(),
        solid({
            ssr,
            serverFunctions: false,
            solid: { moduleName: "@destack/view/runtime" },
            ...(start === undefined ? {} : { start: { ...start, env: start.env ?? false } }),
        }),
    ];
}

/** Refuse Solid server functions and resolve framework imports from this package's installation. */
function frameworkPlugin(): Plugin {
    const importer = fileURLToPath(import.meta.url);
    let isDevelopment = false;

    return {
        name: `${VIEW_PACKAGE}/framework`,
        enforce: "pre",
        configResolved(configuration) {
            isDevelopment = configuration.command === "serve";
        },
        transform: {
            filter: { id: /\.[cm]?[jt]sx?(?:\?|$)/u, code: /["']use server["']/u },
            handler(code, id) {
                // reject server-function directives before framework compilation
                const [path = id] = id.split("?");
                const parsed = parseSync(path, code);
                if (parsed.errors.length) {
                    throw new TypeError(`invalid framework source: ${id}`);
                }
                new Visitor({
                    ExpressionStatement(node) {
                        if (node.directive === "use server") {
                            throw new TypeError(
                                `solid server functions are unsupported: ${id}:${node.start}`,
                            );
                        }
                    },
                }).visit(parsed.program);
            },
        },
        resolveId: {
            filter: { id: /^(?:solid-js|@solidjs\/web)(?:\/|$)/u },
            async handler(source, origin) {
                // resolve generated imports and development refresh modules from this installation
                const isGenerated =
                    origin?.replace(/^\0/u, "").startsWith("virtual:solid-") === true;
                if (!isDevelopment && !isGenerated) {
                    return;
                }

                return await this.resolve(source, importer, { skipSelf: true });
            },
        },
    };
}
