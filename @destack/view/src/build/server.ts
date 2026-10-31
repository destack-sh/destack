import { isAbsolute, join } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import { createServer, type ServerOptions, type ViteDevServer } from "vite";
import type { Workspace } from "@destack/build";
import { loadTransforms } from "@destack/build/local";
import { DeclarationDescription } from "@destack/package";
import type { Plugin } from "@destack/package/build";
import { runtimeConditions, runtimeCss, runtimeTarget } from "@destack/package/build";
import { PackageLocator } from "@destack/package/locator";
import { modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import { present, schema } from "@destack/schema";
import type { PageAssets, StaticRoutes } from "../static/page.ts";
import { ViewError } from "../error/error.ts";
import {
    entryPlugin,
    type LocatedView,
    locate,
    PREFIX,
    type StaticRender,
    type ViewPackage,
} from "./build.ts";

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

    /** Start serving the view with the build's transforms and entries. */
    static async start(options: DevelopmentServerOptions): Promise<DevelopmentServer> {
        // follow definition changes once the server exists
        let server: DevelopmentServer | undefined;
        const follow = (): void => {
            if (server !== undefined) {
                server.#follow();
            }
        };

        // listen, keeping the server
        const vite = await serve(options, options.server, follow);
        server = new DevelopmentServer(vite, options);

        return server;
    }

    /** Replace the Vite server with one of fresh plugins on the same port. */
    restart(): Promise<void> {
        // restart after any pending restart, keeping the chain going past a failed one
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
        // keep the port the browser connects to
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
    readonly workspace: Workspace;
    /** The static view's name. */
    readonly view: string;
    /** Network and filesystem access, listening on loopback by default. */
    readonly server?: ServerOptions;
    /** The clock, files and processes the server compiles with. */
    readonly environment: Environment<"files" | "process">;
}

/** Create a Vite server of fresh plugins for the view and listen with it. */
async function serve(
    options: DevelopmentServerOptions,
    server: ServerOptions | undefined,
    onDefinitionChange: () => void,
): Promise<ViteDevServer> {
    // find the view among the workspace root's declarations
    const { workspace, environment } = options;
    const directory = workspace.root.directory;
    const views = await inspect(workspace);
    const view = views.located.find((each) => each.name === options.view);
    if (view?.render.mode !== "static") {
        throw ViewError.of(
            "VIEW_NOT_STATIC",
            `the package declares no static view ${options.view}`,
        );
    }
    const { render } = view;

    // compile the view with the build's transforms and entries, resolving packages as installed
    const vite = await createServer({
        root: directory,
        configFile: false,
        envDir: false,
        appType: "custom",
        plugins: [
            modulePlugin(new ModuleTransformer(environment.files)),
            definitionPlugin(directory, environment.files, onDefinitionChange),
            ...(await loadTransforms({ directory, runtime: "browser", server: true, environment })),
            entryPlugin(views.located, views.package),
            pagePlugin(view, render),
        ],
        resolve: { conditions: runtimeConditions("browser", "development") },
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

/** Read a workspace root's views, each at the module and symbol declaring it. */
async function inspect(
    workspace: Workspace,
): Promise<{ readonly package: ViewPackage; readonly located: LocatedView[] }> {
    // read the root's declarations from its inspection
    const { root } = workspace;
    const inspection = await workspace.inspect({ runtime: "bun" });
    const { declarations } = schema
        .looseObject({ declarations: schema.array(DeclarationDescription) })
        .parse(inspection.descriptions);

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
        throw new TypeError(`the server entry exports no routes and respond functions: ${entry}`);
    }

    // leave a file path no endpoint answers to Vite
    const { endpoints } = await module.routes();
    const isFile = /\.[^/]+$/u.test(url.pathname);
    if (isFile && !endpoints.includes(url.pathname)) {
        return undefined;
    }

    // answer an endpoint, or the page with its browser entry loading through Vite and its updates
    const assets: PageAssets = { script: `/@id/__x00__${entry}`, stylesheets: [] };
    const response = await module.respond(new Request(url), assets);
    if (isFile) {
        return response;
    }
    const html = (await response.text()).replace("</head>", `${VITE_CLIENT}</head>`);

    return new Response(html, { status: response.status, headers: response.headers });
}

/** Report whether a module exports a static view's routes and respond functions. */
function isServerModule(module: unknown): module is {
    readonly routes: () => Promise<StaticRoutes>;
    readonly respond: (request: Request, assets: PageAssets) => Promise<Response>;
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
        const definition = join(owner, "destack.json");
        if (!watched.has(definition)) {
            watched.add(definition);
            vite.watcher.add(definition);
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
