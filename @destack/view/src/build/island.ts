import { relative, sep } from "node:path";
import { RolldownMagicString } from "rolldown";
import { parseSync } from "rolldown/utils";
import type { Plugin } from "@destack/package/build";
import { ViewError } from "../error/error.ts";
import { VIEW_PACKAGE } from "./plugin.ts";

/** The modules of a package's islands: its own components in `islands/` directories, as Fresh's. */
const ISLAND_MODULE = /\/islands\/[^/]+\.[jt]sx(?:\?|$)/u;

/** Wrap each island module's default export as an island where pages render on the server. */
export function islandPlugin(directory: string): Plugin {
    return {
        name: `${VIEW_PACKAGE}/island`,
        transform: {
            filter: { id: ISLAND_MODULE },
            handler(code, id, options) {
                // leave browser modules, which hydrate the bare component, and a dependency's islands
                const [path = id] = id.split("?");
                if (options?.ssr !== true) {
                    return undefined;
                }
                const module = relative(directory, path).split(sep).join("/");
                if (module.startsWith("..") || module.split("/").includes("node_modules")) {
                    return undefined;
                }

                return wrapIsland(code, path, `/${module}`);
            },
        },
    };
}

/** Wrap an island module's default export, refusing a module without one. */
export function wrapIsland(
    code: string,
    path: string,
    module: string,
): { readonly code: string; readonly map: string } {
    // find the default export
    const parsed = parseSync(path, code);
    if (parsed.errors.length) {
        throw ViewError.of("SOURCE_INVALID", `invalid island source: ${path}`);
    }
    const exported = parsed.program.body.find(
        (statement) => statement.type === "ExportDefaultDeclaration",
    );
    if (exported === undefined) {
        throw ViewError.of("ISLAND_INVALID", `the island ${module} exports no default component`);
    }

    // keep a named declaration as it is, else bind the exported expression as the component
    const source = new RolldownMagicString(code);
    const { declaration } = exported;
    const declared =
        declaration.type === "FunctionDeclaration" || declaration.type === "ClassDeclaration"
            ? declaration.id?.name
            : undefined;
    const name = declared ?? "component";
    if (declared === undefined) {
        source.overwrite(exported.start, declaration.start, "const component = ");
    } else {
        source.remove(exported.start, declaration.start);
    }

    // export the component's island
    source.prepend(`import { island } from "${VIEW_PACKAGE}/static";\n`);
    source.append(`\nexport default island(${name}, ${JSON.stringify(module)});\n`);

    return {
        code: source.toString(),
        map: source.generateMap({ source: path, hires: true }).toString(),
    };
}
