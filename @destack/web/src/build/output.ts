import { readdir, readFile } from "node:fs/promises";
import { join, relative, resolve, sep } from "node:path";
import { createBuilder, type InlineConfig } from "vite";
import {
    HostModules,
    type ExpandedOutput,
    type OutputKind,
    type Pass,
    type Plugin,
    runtimeConditions,
} from "@destack/package/build";
import type { PackageOutput } from "@destack/package/manifest";
import type { Runtime } from "@destack/package/runtime";
import { schema } from "@destack/schema";
import { SolidApplication, VIEW_PACKAGE } from "@destack/view/build";
import { entryPlugin } from "./entry.ts";
import { prerender, PrerenderOptions } from "./prerender/index.ts";
import { sourcePlugin } from "./source.ts";

/** A Solid application compiled into a browser output and, with server rendering, a server output. */
export const WebOptions = SolidApplication.extend({
    /** Compile a browser application and its server handler. */
    kind: schema.literal("web"),
    /** Server runtime and whether to distribute the handler. */
    ssr: schema.union([
        schema.literal(false),
        schema.object({
            /** The runtime serving the handler. */
            runtime: schema.enum(["bun", "workerd"]),
            /** Whether to distribute the handler. */
            emit: schema.boolean().exactOptional(),
        }),
    ]),
    /** Public URL prefix used by browser assets. */
    base: schema.string().exactOptional(),
    /** Minify generated JavaScript. */
    minify: schema.boolean().exactOptional(),
    /** Package-relative public asset directory, public by default. */
    publicDirectory: schema.union([schema.string(), schema.literal(false)]).exactOptional(),
    /** Render these public routes during the build. */
    prerender: PrerenderOptions.exactOptional(),
});
/** A Solid application compiled into a browser output and, with server rendering, a server output. */
export type WebOptions = schema.Infer<typeof WebOptions>;

/** The asset manifest Vite writes for a build, by module key. */
const ViteManifest = schema.record(
    schema.string(),
    schema.looseObject({
        /** The chunk's source module. */
        src: schema.string().exactOptional(),
        /** The emitted file. */
        file: schema.string(),
        /** The stylesheets the chunk loads. */
        css: schema.array(schema.string()).exactOptional(),
        /** The assets the chunk references. */
        assets: schema.array(schema.string()).exactOptional(),
        /** Whether the chunk is an entry. */
        isEntry: schema.boolean().exactOptional(),
        /** The chunk's name. */
        name: schema.string().exactOptional(),
        /** Whether the chunk is a dynamic entry. */
        isDynamicEntry: schema.boolean().exactOptional(),
        /** The chunks the chunk imports. */
        imports: schema.array(schema.string()).exactOptional(),
        /** The chunks the chunk imports dynamically. */
        dynamicImports: schema.array(schema.string()).exactOptional(),
    }),
);

/** The render modes Start knows, any other naming the module choosing one. */
const RENDER_MODES = new Set(["sync", "async", "stream"]);

/** Compile Solid applications through Start's client and server environments. */
export const webOutput: OutputKind = {
    expand(name, request) {
        // require a server renderer for prerendered pages
        const application = WebOptions.parse(request);
        if (application.prerender !== undefined && application.ssr === false) {
            throw new TypeError("prerendering requires an SSR handler");
        }

        // inspect the framework entries beside the application
        const files = frameworkFiles(application);
        const entries = { ".": application.app };
        const browser: ExpandedOutput = { runtime: "browser", entries, files };

        // expand into the browser output and the server output rendering it
        if (application.ssr === false) {
            return { [`${name}-browser`]: browser };
        } else {
            const server = { runtime: application.ssr.runtime, entries, files };

            return { [`${name}-browser`]: browser, [`${name}-server`]: server };
        }
    },
    async compile(name, request, compilations, pass) {
        // stage the package where Start writes its client and server builds
        const application = WebOptions.parse(request);
        const browser = `${name}-browser`;
        const server = application.ssr === false ? undefined : `${name}-server`;
        const compilation = compilations[browser];
        if (compilation === undefined) {
            throw new TypeError(`missing browser compilation: ${browser}`);
        }
        const { directory } = compilation;
        await using stage = await pass.stage();
        const temporary = join(stage.directory, "dist");

        // let Start build the client before the server reads its asset manifest
        const build = { name, application, pass, directory, stage: stage.directory, temporary };
        const builder = await createBuilder(builderConfiguration(build));
        await builder.buildApp();

        // check the server entrypoints against their runtime and the browser code entirely
        if (server !== undefined) {
            const entrypoints = [application.entryServer, application.middleware]
                .filter((path) => path !== undefined)
                .map((path) => relative(directory, resolve(directory, path)).split(sep).join("/"));
            await pass.check(server, entrypoints);
        }
        await pass.check(browser);

        // keep prerendered pages beside the browser assets for static or hybrid hosting
        if (application.prerender !== undefined) {
            const pages = await prerender(
                join(temporary, "server/server.js"),
                application.prerender,
            );
            for (const [path, bytes] of pages) {
                pass.file(`output/${browser}/${path}`, bytes);
            }
        }

        // keep each distributed side's files, describing Start's asset manifest by package paths
        const outputs: Record<string, PackageOutput> = {};
        await keepSide(build, "client", browser);
        outputs[browser] = describeSide(build, "client", browser);
        if (server !== undefined) {
            await keepSide(build, "server", server);
            outputs[server] = describeSide(build, "server", server);
        }

        return outputs;
    },
};

/** One application build: its request, the pass compiling it and the directories Start builds in. */
interface WebBuild {
    /** The requested output's name. */
    readonly name: string;
    /** The application's settings. */
    readonly application: WebOptions;
    /** The pass compiling the application's outputs. */
    readonly pass: Pass;
    /** The package's source directory. */
    readonly directory: string;
    /** The staged package directory Start builds in. */
    readonly stage: string;
    /** The directory Start writes its client and server builds into. */
    readonly temporary: string;
}

/** List the framework modules an application names beside its root component. */
function frameworkFiles(application: WebOptions): string[] {
    const renderMode =
        application.renderMode !== undefined && !RENDER_MODES.has(application.renderMode)
            ? application.renderMode
            : undefined;

    return [
        application.document,
        application.entryClient,
        application.entryServer,
        application.middleware,
        application.setup,
        renderMode,
    ].filter((path): path is string => typeof path === "string");
}

/** Read an application's server runtime, Bun when it renders nothing on a server. */
function serverRuntime(application: WebOptions): Runtime {
    return application.ssr === false ? "bun" : application.ssr.runtime;
}

/** Configure Start's client and server environments for one application build. */
function builderConfiguration(build: WebBuild): InlineConfig {
    // read the application's server runtime and outputs
    const { name, application, pass, directory, temporary } = build;
    const runtime = serverRuntime(application);
    const browser = `${name}-browser`;
    const server = application.ssr === false ? undefined : `${name}-server`;
    const isRendered = server !== undefined || application.prerender !== undefined;

    return {
        root: build.stage,
        configFile: false,
        envDir: false,
        publicDir: application.publicDirectory ?? "public",
        logLevel: "silent",
        define: { "process.env.NODE_ENV": JSON.stringify("production") },
        base: application.base ?? "/",
        ssr: {
            noExternal: true,
            target: runtime === "workerd" ? "webworker" : "node",
            resolve: { conditions: runtimeConditions(runtime) },
        },
        plugins: [
            ...pass.plugins(),
            sourcePlugin(build.stage),
            pass.record("client", browser),
            pass.record("ssr", server),
            ...pass.transforms({
                runtime: "browser",
                server: isRendered,
                options: { [VIEW_PACKAGE]: SolidApplication.strip().parse(application) },
            }),
            entryPlugin(),
            ...environmentPlugins(temporary, application.minify ?? false, runtime),
        ],
        build: {
            sourcemap: true,
            copyPublicDir: true,
            rolldownOptions: {
                cwd: directory,
                experimental: { attachDebugInfo: "none" },
                output: {
                    sourcemapPathTransform(source, map) {
                        // name each map by the output its side writes
                        const generated = relative(temporary, map)
                            .split(sep)
                            .join("/")
                            .replace(/^client\//u, "browser/");

                        return pass.mapSource(source, map, `output/${name}-${generated}`);
                    },
                },
            },
        },
    };
}

/** Write Start's client and server builds into the temporary directory, each resolving for its runtime. */
function environmentPlugins(temporary: string, minify: boolean, runtime: Runtime): Plugin[] {
    // resolve the host modules of the browser and of the server runtime
    const isWorkerd = runtime === "workerd";
    const browserHosts = new HostModules("browser");
    const serverHosts = new HostModules(runtime);

    return [
        { ...browserHosts.plugin(), applyToEnvironment: ({ name }) => name === "client" },
        { ...serverHosts.plugin(), applyToEnvironment: ({ name }) => name === "ssr" },
        {
            name: "@destack/web/output",
            enforce: "post",
            config: () => ({
                environments: {
                    client: {
                        build: {
                            outDir: join(temporary, "client"),
                            minify,
                            rolldownOptions: {
                                resolve: { conditionNames: runtimeConditions("browser") },
                                external: (specifier: string, importer: string | undefined) =>
                                    browserHosts.resolve(specifier, importer) === "external",
                            },
                        },
                    },
                    ssr: {
                        resolve: { conditions: runtimeConditions(runtime) },
                        build: {
                            outDir: join(temporary, "server"),
                            emitAssets: true,
                            copyPublicDir: false,
                            minify,
                            rolldownOptions: {
                                resolve: {
                                    mainFields: isWorkerd
                                        ? ["browser", "module", "main"]
                                        : ["module", "main"],
                                    conditionNames: runtimeConditions(runtime),
                                },
                                external: (specifier: string, importer: string | undefined) =>
                                    serverHosts.resolve(specifier, importer) === "external",
                                platform: isWorkerd ? "browser" : "node",
                            },
                        },
                    },
                },
            }),
        },
    ];
}

/** Report whether an application distributes its server handler. */
function isEmitted(application: WebOptions): boolean {
    return application.ssr !== false && application.ssr.emit !== false;
}

/** Keep a distributed side's files, describing Start's asset manifest by package paths. */
async function keepSide(build: WebBuild, side: "client" | "server", output: string): Promise<void> {
    // read the side's files, the server's only when distributed
    const { pass } = build;
    const root = join(build.temporary, side);
    if (side === "server" && !isEmitted(build.application)) {
        return;
    }
    const entries = await readdir(root, { recursive: true, withFileTypes: true });

    // keep each file, describing the asset manifest and its source maps
    for (const entry of entries.filter((candidate) => candidate.isFile())) {
        const path = join(entry.parentPath, entry.name);
        const local = relative(root, path).replaceAll("\\", "/");
        const generated = `output/${output}/${local}`;
        if (local === ".vite/manifest.json") {
            const manifest = ViteManifest.parse(JSON.parse(await readFile(path, "utf8")));
            const described = pass.describeAssets(manifest, build.stage);
            pass.file(generated, new TextEncoder().encode(JSON.stringify(described)));
        } else {
            await pass.copy(generated, path);
        }
        if (local.endsWith(".js.map")) {
            pass.sourceMap(generated.slice(0, -4), generated);
        }
    }
}

/** Describe a side's output, distributing the server handler only when emitted. */
function describeSide(build: WebBuild, side: "client" | "server", output: string): PackageOutput {
    // describe the side's runtime, distribution and exports
    const { application } = build;
    const isServer = side === "server";
    const emitted = isEmitted(application);

    return {
        runtime: isServer ? serverRuntime(application) : "browser",
        emit: !isServer || emitted,
        workloads: {},
        views: {},
        tests: [],
        directory: `output/${output}`,
        exports: sideExports(application, side, output),
        dependencies: {},
    };
}

/** Export a server's handler when distributed, and a browser page when nothing renders it on a server. */
function sideExports(
    application: WebOptions,
    side: "client" | "server",
    output: string,
): Record<string, string> {
    // export the distributed server handler
    if (side === "server") {
        return isEmitted(application) ? { ".": `output/${output}/server.js` } : {};
    }
    // export the browser page of an application without server rendering
    else if (application.ssr === false && application.prerender === undefined) {
        return { ".": `output/${output}/index.html` };
    }
    // export nothing from a server-rendered browser output
    else {
        return {};
    }
}
