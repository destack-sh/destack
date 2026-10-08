import { isAbsolute, join } from "node:path";
import { createServer, type ServerOptions, type ViteDevServer } from "vite";
import { readDependencies } from "@destack/build";
import {
    loadTransforms,
    openSource,
    type PackageSource,
    resolutionPlugin,
} from "@destack/build/local";
import type { DependencyResolution } from "@destack/package";
import type { Plugin } from "@destack/package/build";
import { runtimeConditions } from "@destack/package/build";
import { PackageLocator } from "@destack/package/transform";
import { modulePlugin } from "@destack/package/vite";
import { SolidApplication, VIEW_PACKAGE } from "@destack/view/build";
import { entryPlugin } from "./entry.ts";
import { writeMetadata } from "./metadata/index.ts";
import { requireSite, type WebOptions } from "./output.ts";

/** The browsers development serves syntax for, the ones Vite's `baseline-widely-available` build target names. */
const BROWSER_TARGETS = ["chrome111", "edge111", "firefox114", "safari16.4"];

/** A local application server with Vite's watcher and module graph, restarted when a package definition it reads changes. */
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
    /** Whether compiler configuration files have been removed. */
    #isReleased = false;

    /** Retain the server and package until shutdown. */
    private constructor(
        vite: ViteDevServer,
        options: DevelopmentServerOptions,
        source: PackageSource,
    ) {
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
        // open the application's package source
        const source = await open(options);
        let server: DevelopmentServer | undefined;
        let vite: ViteDevServer | undefined;

        // follow definition changes once the server exists
        const follow = (): void => {
            if (server !== undefined) {
                server.#follow();
            }
        };

        // release both resources if configuration or startup fails
        try {
            vite = await serve(options, source, options.server, follow);
            server = new DevelopmentServer(vite, options, source);

            return server;
        } catch (error) {
            try {
                await vite?.close();
            } finally {
                await source[Symbol.asyncDispose]();
            }
            throw error;
        }
    }

    /** Replace the Vite server with one of fresh plugins on the same port, as Vite restarts on a configuration change. */
    restart(): Promise<void> {
        // restart after any pending restart, keeping the chain going past a failed one
        const restarted = this.#restarting.then(() => this.#replace());
        this.#restarting = restarted.catch(() => undefined);

        return restarted;
    }

    /** Close network listeners, watchers, and compiler configuration. */
    async close(): Promise<void> {
        // stop further restarts and close the listeners and watchers once after a pending restart
        const isClosing = !this.#isClosed;
        this.#isClosed = true;
        try {
            if (isClosing) {
                await this.#restarting;
                await this.#vite.close();
            }
        } finally {
            // remove the compiler configuration once
            if (!this.#isReleased) {
                this.#isReleased = true;
                await this.#source[Symbol.asyncDispose]();
            }
        }
    }

    /** Close the local server. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Close the running Vite server and listen with a fresh one on its port, unless the server closed. */
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
        await this.#source[Symbol.asyncDispose]();
        this.#isReleased = true;
        this.#source = await open(this.#options);
        this.#isReleased = false;

        // listen again with fresh plugins, which read every dependency's definition anew
        this.#vite = await serve(this.#options, this.#source, settings, () => this.#follow());
    }

    /** Restart once a package definition changes, reporting a failed restart in the server's log. */
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
    /** The dependency releases to resolve in place of the ones the package's lockfile selects, such as a test's. */
    dependencies?: Readonly<Record<string, DependencyResolution>>;
    /** The application's browser and server settings. */
    application: WebOptions;
    /** The package's TypeScript configuration. */
    configuration?: string;
    /** Network and filesystem access, listening on loopback by default. */
    server?: ServerOptions;
}

/** Open the application's package source with its selected entry. */
async function open(options: DevelopmentServerOptions): Promise<PackageSource> {
    return await openSource({
        directory: options.directory,
        runtime: "browser",
        ...(options.configuration === undefined ? {} : { configuration: options.configuration }),
        entries: { ".": options.application.app },
    });
}

/** Create and listen with a Vite server of fresh plugins for an application, calling back when a dependency's definition changes. */
async function serve(
    options: DevelopmentServerOptions,
    source: PackageSource,
    server: ServerOptions | undefined,
    onDefinitionChange: () => void,
): Promise<ViteDevServer> {
    // compile the application with the build's plugins and transforms, resolving the releases its lockfile selects
    const application = options.application;
    const runtime = application.ssr === false ? "browser" : application.ssr.runtime;
    const dependencies = options.dependencies ?? (await readDependencies(source.directory));
    const vite = await createServer({
        root: source.directory,
        configFile: false,
        envDir: false,
        publicDir: application.publicDirectory ?? "public",
        base: application.base ?? "/",
        plugins: [
            modulePlugin(),
            resolutionPlugin(source, dependencies, runtime),
            definitionPlugin(source.directory, onDefinitionChange),
            metadataPlugin(application),
            ...(await loadTransforms({
                directory: source.directory,
                runtime: "browser",
                server: application.ssr !== false,
                options: { [VIEW_PACKAGE]: SolidApplication.strip().parse(application) },
            })),
            entryPlugin(),
        ],
        resolve: { conditions: runtimeConditions("browser", "development") },
        oxc: { target: BROWSER_TARGETS },
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

/** Watch the application's definition and each one of a package whose modules the server loads, calling back once one changes or goes away. */
function definitionPlugin(directory: string, onChange: () => void): Plugin {
    // find packages afresh for this server and remember the definitions watched
    const packages = new PackageLocator();
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

/** Serve the site files an application publishes, written afresh for each request as builds write them. */
function metadataPlugin(application: WebOptions): Plugin {
    return {
        name: "@destack/web/metadata",
        configureServer(vite) {
            // leave an application without site files
            const metadata = application.metadata;
            if (metadata === undefined) {
                return;
            }
            const site = requireSite(application);

            // answer a request for a site file, passing every other request on
            vite.middlewares.use((request, response, next) => {
                const path = request.url?.replace(/\?.*$/su, "");
                const files = writeMetadata(site, metadata, new Date());
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
