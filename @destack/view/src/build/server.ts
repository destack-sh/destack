import { isAbsolute, join, relative } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import { createServer, isCSSRequest, type ServerOptions, type ViteDevServer } from "vite";
import type { WorkspacePackage } from "@destack/build";
import { loadTransforms } from "@destack/build/local";
import { OriginPath } from "@destack/package";
import { type Plugin, runtimeConditions, runtimeCss, runtimeTarget } from "@destack/package/build";
import { PackageLocator } from "@destack/package/locator";
import { baselinePlugin, modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import { present } from "@destack/schema";
import type { PageHead, RouteTable } from "../static/page.ts";
import { ViewError } from "../error/error.ts";
import {
    entryPlugin,
    type LocatedView,
    locate,
    PREFIX,
    type StaticRender,
    type ViewPackage,
} from "./build.ts";
import { examplesPlugin, readExamples } from "./examples.ts";
import { jsonPlugin } from "./json.ts";
import { ORIGIN, originPlugin } from "./origin.ts";

/** The files that define a package: its declarations, and the entries it exports. */
const DEFINITIONS = ["destack.json", "package.json"];

/** The module hosting examples, which their page loads. */
const EXAMPLE_HOST = join(import.meta.dirname, "../example/index.ts");

/** The script connecting a page to Vite's updates, loaded after the head the page hydrates. */
const VITE_CLIENT = '<script type="module" src="/@vite/client"></script>';

/** A local server of a workspace's static view on Vite, rendering each page on request. */
export class DevelopmentServer implements AsyncDisposable {
    /** The running Vite server, replaced on each restart. */
    #vite: ViteDevServer;
    /** The options the server starts from, applied again on each restart. */
    readonly #options: DevelopmentServerOptions;
    /** The restarts in order, settled once the last one has. */
    #restarting: Promise<void> = Promise.resolve();
    /** Whether network listeners and watchers have closed. */
    #isClosed = false;

    /** Retain the server until shutdown. */
    private constructor(vite: ViteDevServer, options: DevelopmentServerOptions) {
        this.#vite = vite;
        this.#options = options;
    }

    /** The running Vite server. */
    get vite(): ViteDevServer {
        return this.#vite;
    }

    /** Serve the package's view, when it names one, and its examples. */
    static async start(options: DevelopmentServerOptions): Promise<DevelopmentServer> {
        // follow definition changes once the server exists
        let server: DevelopmentServer | undefined;
        const follow = (): void => {
            if (server !== undefined) {
                server.#follow();
            }
        };

        // listen, then create the server around Vite
        const vite = await serve(options, options.server, follow);
        server = new DevelopmentServer(vite, options);

        return server;
    }

    /** Replace the Vite server with one of fresh plugins on the same port. */
    restart(): Promise<void> {
        // restart after any pending restart, continuing the chain past a failed one
        const restarted = this.#restarting.then(() => this.#replace());
        this.#restarting = restarted.catch(() => undefined);

        return restarted;
    }

    /** Close network listeners and watchers. */
    async close(): Promise<void> {
        // stop further restarts and close the listeners and watchers once after a pending restart
        if (!this.#isClosed) {
            this.#isClosed = true;
            await this.#restarting;
            await this.#vite.close();
        }
    }

    /** Close the local server. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Close the running Vite server and listen with a fresh one on its port, unless closed. */
    async #replace(): Promise<void> {
        // stop once closed, else reuse the port the browser connects to
        if (this.#isClosed) {
            return;
        }
        const address = this.#vite.httpServer?.address();
        const settings =
            typeof address === "object" && address !== null
                ? { ...this.#options.server, port: address.port, strictPort: true }
                : this.#options.server;
        await this.#vite.close();

        // listen again with fresh plugins, which read the declarations and every definition anew
        this.#vite = await serve(this.#options, settings, () => this.#follow());
    }

    /** Restart when a package definition changes, reporting failures in the server's log. */
    #follow(): void {
        const logger = this.#vite.config.logger;
        logger.info("package definition changed, restarting the server");
        this.restart().catch((error: unknown) => {
            logger.error(`failed to restart the server: ${String(error)}`, {
                error: error instanceof Error ? error : null,
            });
        });
    }
}

/** The workspace, view and network settings of a local view server. */
export interface DevelopmentServerOptions {
    /** The workspace whose root declares the view. */
    readonly workspace: WorkspacePackage;
    /** The static view's name, none for a package that only shows its examples. */
    readonly view?: string;
    /** Network and filesystem access, listening on loopback by default. */
    readonly server?: ServerOptions;
    /** The clock, files and processes the server compiles with. */
    readonly environment: Environment<"files" | "process">;
}

/** Create a Vite server for the view and the examples. */
async function serve(
    options: DevelopmentServerOptions,
    server: ServerOptions | undefined,
    onDefinitionChange: () => void,
): Promise<ViteDevServer> {
    // answer the examples below their reserved path, and the view's pages when it names one
    const { workspace, environment, view } = options;
    const directory = workspace.root.directory;
    const views = await readViews(workspace);
    const examples = examplesPlugin(async () => readExamples(workspace));
    const pages =
        view === undefined ? [rootPlugin(OriginPath.examples)] : [viewPlugin(views.located, view)];

    // compile the view with the build's transforms and entries, resolving packages as installed
    const vite = await createServer({
        root: directory,
        configFile: false,
        envDir: false,
        appType: "custom",
        plugins: [
            modulePlugin(new ModuleTransformer(environment)),
            baselinePlugin(),
            jsonPlugin(),
            definitionPlugin(directory, environment.files, onDefinitionChange),
            ...(await loadTransforms({ directory, runtime: "browser", server: true, environment })),
            entryPlugin(views.located, views.package),
            examples,
            originPlugin(environment),
            ...pages,
        ],
        resolve: { conditions: runtimeConditions("browser", "development") },
        // bundle the npm packages of every example and view at startup, as one found later reloads pages
        optimizeDeps: { entries: await entriesOf(workspace, views.located) },
        // serve the syntax and stylesheets of the baseline browsers, as builds emit them
        oxc: { target: runtimeTarget("browser") },
        css: runtimeCss("browser"),
        // compile the workspace's Destack packages as the build does, and load npm packages as installed
        ssr: {
            noExternal: workspace.units.map((unit) => unit.name),
            resolve: { conditions: runtimeConditions("bun", "development") },
        },

        server: { host: "127.0.0.1", ...server },
    });

    // close the server again when it cannot listen
    try {
        await vite.listen();
    } catch (error) {
        await vite.close();
        throw error;
    }

    return vite;
}

/** List the modules pages load, relative to the workspace root: examples, their host and views. */
async function entriesOf(
    workspace: WorkspacePackage,
    views: readonly LocatedView[],
): Promise<string[]> {
    const examples = await readExamples(workspace);

    return [
        ...new Set([
            ...examples.map((example) => example.module.slice(1)),
            ...[EXAMPLE_HOST, ...views.map((view) => view.file)].map((file) =>
                relative(workspace.root.directory, file),
            ),
        ]),
    ];
}

/** Read a workspace root's views, each at the module and symbol declaring it. */
async function readViews(
    workspace: WorkspacePackage,
): Promise<{ readonly package: ViewPackage; readonly located: LocatedView[] }> {
    // evaluate the root's declarations from source
    const { root } = workspace;
    const declarations = await workspace.declarations({ runtime: "bun" });

    // locate each at the module and export declaring it
    const owned: ViewPackage = {
        package: root.packageJson.package,
        declarations,
        locate: ({ symbol }) => ({
            file: join(root.directory, symbol.symbol.module),
            export: symbol.symbol.name,
        }),
    };

    return { package: owned, located: locate(owned) };
}

/** Send the root to another path, as a package without a view opens on its examples. */
function rootPlugin(path: string): Plugin {
    return {
        name: "@destack/view/root",
        configureServer(vite) {
            vite.middlewares.use((request, response, next) => {
                const { pathname } = new URL(present(request.url, "the request's path"), ORIGIN);

                // redirect the root
                if (pathname === "/") {
                    response.statusCode = 302;
                    response.setHeader("location", path);
                    response.end();
                }
                // leave every other path to the origin, Vite and the examples
                else {
                    next();
                }
            });
        },
    };
}

/** Answer the pages of a static view. */
function viewPlugin(located: readonly LocatedView[], name: string): Plugin {
    const view = located.find((each) => each.name === name);
    if (view?.render.mode !== "static") {
        throw ViewError.of(
            "VIEW_INVALID",
            `static view '${name}' is invalid: the package declares no such static view`,
        );
    }

    return pagePlugin(view, view.render);
}

/** Answer the view's endpoints and its pages for document requests, as its prerender does. */
function pagePlugin(view: LocatedView, render: StaticRender): Plugin {
    return {
        name: "@destack/view/page",
        configureServer(vite) {
            // answer after Vite's own modules and files
            return () => {
                vite.middlewares.use((request, response, next) => {
                    const url = new URL(present(request.url, "the request's path"), render.origin);
                    respond(vite, view, url).then(
                        async (answered) => {
                            // pass a request the view answers nothing for on
                            if (answered === undefined) {
                                next();

                                return;
                            }
                            response.statusCode = answered.status;
                            for (const [name, value] of answered.headers) {
                                response.setHeader(name, value);
                            }
                            response.end(Buffer.from(await answered.arrayBuffer()));
                        },
                        (error: unknown) => {
                            next(error);
                        },
                    );
                });
            };
        },
    };
}

/** Answer an endpoint or a page of the view at a URL, absent for a file Vite answers. */
async function respond(
    vite: ViteDevServer,
    view: LocatedView,
    url: URL,
): Promise<Response | undefined> {
    // load the view's server entry through Vite
    const entry = `${PREFIX}${view.name}`;
    const module: unknown = await vite.ssrLoadModule(entry);
    if (!isServerModule(module)) {
        throw new TypeError(`the server entry exports no routes and respond functions: '${entry}'`);
    }

    // leave a file path no endpoint answers to Vite
    const { endpoints } = await module.routes();
    const isFile = /\.[^/]+$/u.test(url.pathname);
    if (isFile && !endpoints.includes(url.pathname)) {
        return undefined;
    }

    // answer an endpoint, or the page with its browser entry, the stylesheets it renders, and Vite's updates
    const stylesheets = await renderedStylesheets(vite, entry);
    const head: PageHead = {
        script: `/@id/__x00__${entry}`,
        stylesheets,
        telemetry: OriginPath.telemetry,
    };
    const response = await module.respond(new Request(url), head);
    if (isFile) {
        return response;
    }
    const html = (await response.text()).replace("</head>", `${VITE_CLIENT}</head>`);

    return new Response(html, { status: response.status, headers: response.headers });
}

/** List the stylesheets a server entry imports, as the build links them. */
async function renderedStylesheets(vite: ViteDevServer, entry: string): Promise<string[]> {
    // start from the entry the server loaded
    const modules = vite.environments.ssr.moduleGraph;
    const root = await modules.getModuleByUrl(entry);
    if (root === undefined) {
        throw new TypeError(`the server loaded no module for its entry: '${entry}'`);
    }

    // collect each reached stylesheet file, following imports, served as a stylesheet instead of a module
    const reached = [root];
    const visited = new Set([root]);
    const stylesheets: string[] = [];
    for (const module of reached) {
        if (isCSSRequest(module.url) && module.url.startsWith("/")) {
            stylesheets.push(`${module.url}${module.url.includes("?") ? "&" : "?"}direct`);
        }
        for (const imported of module.importedModules) {
            if (!visited.has(imported)) {
                visited.add(imported);
                reached.push(imported);
            }
        }
    }

    return stylesheets;
}

/** Report whether a module exports a static view's routes and respond functions. */
function isServerModule(module: unknown): module is {
    readonly routes: () => Promise<RouteTable>;
    readonly respond: (request: Request, head: PageHead) => Promise<Response>;
} {
    return (
        typeof module === "object" &&
        module !== null &&
        "routes" in module &&
        typeof module.routes === "function" &&
        "respond" in module &&
        typeof module.respond === "function"
    );
}

/** Watch the definitions of the package and of each package whose modules the server loads. */
function definitionPlugin(directory: string, files: FileSystem, onChange: () => void): Plugin {
    // find packages afresh for this server and remember the definitions watched
    const packages = new PackageLocator(files);
    const watched = new Set<string>();
    let server: ViteDevServer | undefined;
    const watch = (vite: ViteDevServer, owner: string) => {
        for (const name of DEFINITIONS) {
            const definition = join(owner, name);
            if (!watched.has(definition)) {
                watched.add(definition);
                vite.watcher.add(definition);
            }
        }
    };

    return {
        name: "@destack/view/definition",
        configureServer(vite) {
            // watch the package's definition, calling back on a change to any watched one
            server = vite;
            watch(vite, directory);
            vite.watcher.on("all", (event, path) => {
                if ((event === "change" || event === "unlink") && watched.has(path)) {
                    onChange();
                }
            });
        },
        async load(id) {
            // watch the definition of a loaded module's package, leaving virtual modules
            const [path = id] = id.split("?");
            const owner = isAbsolute(path) ? await packages.find(path) : undefined;
            if (server !== undefined && owner !== undefined) {
                watch(server, owner.directory);
            }

            return null;
        },
    };
}
