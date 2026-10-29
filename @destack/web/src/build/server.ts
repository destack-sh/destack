import { isAbsolute, join } from "node:path";
import type { Clock, Environment, FileSystem } from "@destack/environment";
import { createServer, type ServerOptions, type ViteDevServer } from "vite";
import { loadTransforms, PackageSource } from "@destack/build/local";
import type { Plugin } from "@destack/package/build";
import { runtimeConditions, runtimeCss, runtimeTarget } from "@destack/package/build";
import { PackageLocator } from "@destack/package/locator";
import { modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import { SolidApplication, VIEW_PACKAGE } from "@destack/view/build";
import { entryPlugin } from "./entry.ts";
import { writeMetadata } from "./metadata/index.ts";
import { requireSite, type WebOptions } from "./output.ts";

/** A local application server on Vite, serving workspace packages from source. */
export class DevelopmentServer implements AsyncDisposable {
    /** The running Vite server, replaced on each restart. */
    #vite: ViteDevServer;
    /** The options the server starts from, applied again on each restart. */
    readonly #options: DevelopmentServerOptions;
    /** The retained package configuration, reopened on each restart. */
    #source: PackageSource;
    /** The restarts in order, settled once the last one has. */
    #restarting: Promise<void> = Promise.resolve();
    /** Whether network listeners and watchers have closed. */
    #isClosed = false;

    /** Retain the server and package until shutdown. */
    private constructor(
        vite: ViteDevServer,
        options: DevelopmentServerOptions,
        source: PackageSource,
    ) {
        // keep the server and its source
        this.#vite = vite;
        this.#options = options;
        this.#source = source;
    }

    /** The running Vite server. */
    get vite(): ViteDevServer {
        return this.#vite;
    }

    /** Start the application with the build's JSX, style, and metadata transforms. */
    static async start(options: DevelopmentServerOptions): Promise<DevelopmentServer> {
        // open the application's package source, its workspace dependencies served from theirs
        const source = await open(options);
        let server: DevelopmentServer | undefined;
        let vite: ViteDevServer | undefined;

        // follow definition changes once the server exists
        const follow = (): void => {
            if (server !== undefined) {
                server.#follow();
            }
        };

        // listen, closing the server if startup fails
        try {
            vite = await serve(options, source, options.server, follow);
            server = new DevelopmentServer(vite, options, source);

            return server;
        } catch (error) {
            await vite?.close();
            throw error;
        }
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

        // reopen the package source, which reads the application's own definition anew
        this.#source = await open(this.#options);

        // listen again with fresh plugins, which read every dependency's definition anew
        this.#vite = await serve(this.#options, this.#source, settings, () => this.#follow());
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

/** Source and network settings for a local web application. */
export interface DevelopmentServerOptions {
    /** The source package directory. */
    directory: string;
    /** The application's browser and server settings. */
    application: WebOptions;
    /** The package's TypeScript configuration. */
    configuration?: string;
    /** Network and filesystem access, listening on loopback by default. */
    server?: ServerOptions;
    /** The clock, files and processes the server compiles with. */
    environment: Environment<"files" | "process">;
}

/** Open the application's package source with its selected entry. */
async function open(options: DevelopmentServerOptions): Promise<PackageSource> {
    return await PackageSource.open(
        {
            directory: options.directory,
            runtime: "browser",
            ...(options.configuration === undefined
                ? {}
                : { configuration: options.configuration }),
            entries: { ".": options.application.app },
            tree: {},
        },
        options.environment,
    );
}

/** Create a Vite server of fresh plugins for an application and listen with it. */
async function serve(
    options: DevelopmentServerOptions,
    source: PackageSource,
    server: ServerOptions | undefined,
    onDefinitionChange: () => void,
): Promise<ViteDevServer> {
    // compile the application with the build's plugins and transforms, resolving packages as installed
    const application = options.application;
    const runtime = application.ssr === false ? "browser" : application.ssr.runtime;
    const vite = await createServer({
        root: source.directory,
        configFile: false,
        envDir: false,
        publicDir: application.publicDirectory ?? "public",
        base: application.base ?? "/",
        plugins: [
            modulePlugin(new ModuleTransformer(options.environment.files)),
            definitionPlugin(source.directory, options.environment.files, onDefinitionChange),
            metadataPlugin(application, options.environment.clock),
            ...(await loadTransforms({
                directory: source.directory,
                runtime: "browser",
                server: application.ssr !== false,
                options: { [VIEW_PACKAGE]: SolidApplication.strip().parse(application) },
                environment: options.environment,
            })),
            entryPlugin(),
        ],
        resolve: { conditions: runtimeConditions("browser", "development") },
        // serve the syntax and stylesheets of the baseline browsers, as builds emit them
        oxc: { target: runtimeTarget("browser") },
        css: runtimeCss("browser"),
        ssr: {
            noExternal: true,
            resolve: { conditions: runtimeConditions(runtime, "development") },
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

/** Watch the definitions of the application and of each package whose modules the server loads. */
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
        name: "@destack/web/definition",
        configureServer(vite) {
            // watch the application's definition, calling back on a change to any watched one
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

/** Serve the site files an application publishes, written per request as builds write them. */
function metadataPlugin(application: WebOptions, clock: Clock): Plugin {
    return {
        name: "@destack/web/metadata",
        configureServer(vite) {
            // leave an application without site files
            const metadata = application.metadata;
            if (metadata === undefined) {
                return;
            }

            // read the application's site
            const site = requireSite(application);

            // answer a request for a site file, passing every other request on
            vite.middlewares.use((request, response, next) => {
                // find the site file the request names
                const path = request.url?.replace(/\?.*$/su, "");
                const files = writeMetadata(site, metadata, new Date(clock.now()));
                const file = files.find((candidate) => candidate.path === path);
                if (file === undefined) {
                    next();

                    return;
                }
                response.setHeader("Content-Type", `${file.type}; charset=utf-8`);
                response.end(file.text);
            });
        },
    };
}
