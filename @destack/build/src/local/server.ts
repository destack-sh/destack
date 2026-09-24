import { createServer, type ServerOptions, type ViteDevServer } from "vite";
import { openPackage, type PackageSource } from "../source/index.ts";
import type { ApplicationOptions } from "../compile/application.ts";
import { compilationPlugins } from "../compile/compiler.ts";
import { resolutionPlugin } from "../compile/dependency.ts";
import { runtimeConditions } from "../compile/runtime.ts";
import type { DependencyResolution } from "@destack/package";
import { resolveView } from "../source/view.ts";

/** A local application server with Vite's watcher and module graph. */
export class LocalServer implements AsyncDisposable {
    /** The running Vite server. */
    readonly vite: ViteDevServer;
    /** The retained package configuration. */
    readonly #source: PackageSource;
    /** Whether network listeners and watchers have closed. */
    #closed = false;
    /** Whether compiler configuration files have been removed. */
    #released = false;

    /** Retain the server and package until shutdown. */
    private constructor(vite: ViteDevServer, source: PackageSource) {
        this.vite = vite;
        this.#source = source;
    }

    /** Start the application with the build's JSX, style, and metadata transforms. */
    static async start(options: LocalServerOptions): Promise<LocalServer> {
        // resolve the view and open its package source
        const application = await resolveView(options.directory, options.application);
        const runtime = application.ssr === false ? "browser" : application.ssr.runtime;
        const source = await openPackage({
            directory: options.directory,
            target: "browser",
            configuration: options.configuration,
            entries: { ".": application.app ?? "src/app.tsx" },
        });
        let vite: ViteDevServer | undefined;

        // release both resources if configuration or startup fails
        try {
            vite = await createServer({
                root: source.directory,
                configFile: false,
                envDir: false,
                publicDir: application.publicDirectory ?? "public",
                base: application.base ?? "/",
                plugins: [
                    resolutionPlugin(source, options.dependencies, runtime),
                    ...compilationPlugins(source, application.ssr !== false, application),
                ],
                resolve: { conditions: runtimeConditions("browser", "development") },
                ssr: {
                    noExternal: true,
                    resolve: { conditions: runtimeConditions(runtime, "development") },
                },
                server: { host: "127.0.0.1", ...options.server },
            });
            await vite.listen();

            return new LocalServer(vite, source);
        } catch (error) {
            try {
                await vite?.close();
            } finally {
                await source[Symbol.asyncDispose]();
            }
            throw error;
        }
    }

    /** Close network listeners, watchers, and compiler configuration. */
    async close(): Promise<void> {
        if (!this.#closed) {
            await this.vite.close();
            this.#closed = true;
        }

        if (!this.#released) {
            await this.#source[Symbol.asyncDispose]();
            this.#released = true;
        }
    }

    /** Close the local server. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }
}

/** Serve a package with Vite's source watcher and hot module replacement. */
export async function servePackage(options: LocalServerOptions): Promise<LocalServer> {
    return await LocalServer.start(options);
}

/** Source and network settings for a local web application. */
export interface LocalServerOptions {
    /** The source package directory. */
    directory: string;
    /** Immutable dependency releases selected by the host. */
    dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The application's browser and server settings. */
    application: ApplicationOptions;
    /** The package's TypeScript configuration. */
    configuration?: string;
    /** Network and filesystem access; listens on loopback by default. */
    server?: ServerOptions;
}
