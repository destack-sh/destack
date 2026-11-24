import { graph, OriginPath } from "@destack/package";
import type { Plugin } from "@destack/package/build";
import { present } from "@destack/schema";
import type { WorkspacePackage } from "@destack/build";
import { ORIGIN } from "./origin.ts";

/** The module the examples page reads its examples from. */
const EXAMPLES_MODULE = "virtual:destack/examples";

/** The package's catalogs, its own and those of its dependencies, which the page translates by. */
const CATALOGS = `Object.values(import.meta.glob("/locale/**/*.json", { eager: true, import: "default" }))`;

/** The modules that declare examples. */
const EXAMPLE_MODULE = /\.example\.tsx?$/u;

/** The module, export and name of an example. */
export interface ExampleLocation {
    /** The URL path of the module declaring it, such as /src/board/board.example.tsx. */
    readonly module: string;
    /** The export of the module. */
    readonly export: string;
    /** The example's name, such as `note-toolbar`. */
    readonly name: string;
}

/** Read the examples of a workspace root. */
export async function readExamples(workspace: WorkspacePackage): Promise<ExampleLocation[]> {
    // index the root's browser modules for their examples
    const declared = { declarations: [], relationships: [] };
    const modules = await workspace.index({ runtime: "browser" }, declared);

    return modules.flatMap((module) =>
        module.declarations
            .filter((declaration) => declaration.kind === "example")
            .map((declaration) => ({
                module: `/${graph.Moniker.module(declaration.symbol)}`,
                export: present(graph.Moniker.name(declaration.symbol), "an example's symbol"),
                name: declaration.name,
            })),
    );
}

/** The examples page, which lists the examples of the module the server writes. */
const EXAMPLES_PAGE = `<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Examples</title><link rel="icon" href="data:,"></head>
<body>
<script type="module">
import { BrowserEnvironment } from "@destack/environment/browser";
import { hostExamples, hostScenario } from "@destack/view/example";
import examples, { catalogs } from "${EXAMPLES_MODULE}";
const query = new URLSearchParams(location.search);
const scenario = query.get("scenario");
const name = query.get("name");
if (scenario === null) {
    hostExamples(document.body, examples, BrowserEnvironment.system, catalogs);
} else if (name === null) {
    await import(/* @vite-ignore */ scenario);
} else {
    await hostScenario(document.body, scenario, name, BrowserEnvironment.system);
}
</script>
</body>
</html>`;

/** Answer the examples page below its reserved path, and its module of the examples. */
export function examplesPlugin(read: () => Promise<readonly ExampleLocation[]>): Plugin {
    // list the examples once, until a module that declares some changes
    const resolved = `\0${EXAMPLES_MODULE}`;
    let listed: Promise<readonly ExampleLocation[]> | undefined;

    return {
        name: "@destack/view/examples",
        resolveId(id) {
            return id === EXAMPLES_MODULE ? resolved : null;
        },
        async load(id) {
            // write the module of the examples as they stand, and of the catalogs
            if (id !== resolved) {
                return null;
            }
            listed ??= read();

            // list again on the next request once reading the examples failed
            try {
                return `export default ${JSON.stringify(await listed)};\nexport const catalogs = ${CATALOGS};`;
            } catch (error) {
                listed = undefined;
                throw error;
            }
        },
        configureServer(vite) {
            // list the examples again once a module that declares some comes, goes or changes
            vite.watcher.on("all", (_event, path) => {
                if (EXAMPLE_MODULE.test(path)) {
                    // write the module anew and load the page again
                    listed = undefined;
                    const module = vite.moduleGraph.getModuleById(resolved);
                    if (module !== undefined) {
                        vite.moduleGraph.invalidateModule(module);
                    }
                    vite.ws.send({ type: "full-reload" });
                }
            });

            // answer the examples page before the origin refuses its reserved path
            vite.middlewares.use((request, response, next) => {
                const { pathname } = new URL(present(request.url, "the request's path"), ORIGIN);
                const isExamples =
                    pathname === OriginPath.examples ||
                    pathname.startsWith(`${OriginPath.examples}/`);

                // answer the examples page, transformed as Vite serves its pages
                if (isExamples) {
                    vite.transformIndexHtml(pathname, EXAMPLES_PAGE).then(
                        (html) => {
                            response.setHeader("content-type", "text/html; charset=utf-8");
                            response.end(html);
                        },
                        (error: unknown) => {
                            next(error);
                        },
                    );
                }
                // leave every other path to the origin, Vite and the view
                else {
                    next();
                }
            });
        },
    };
}
