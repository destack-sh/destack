import {
    readdir,
    readFile,
    realpath,
    mkdtemp,
    rm,
    symlink,
    writeFile,
    copyFile,
    mkdir,
} from "node:fs/promises";
import { join, relative, resolve, sep, dirname } from "node:path";
import { createBuilder } from "vite";
import { type StartOptions } from "@solidjs/vite-plugin";
import { type PackageOutput } from "@destack/package/manifest";
import { type BuildDescription } from "@destack/package/inspect";
import { type SourceMapReference } from "@destack/package/source";
import { type PackageSource } from "../source/index.ts";
import { type CompileOptions } from "./module.ts";
import { compilationPlugins, virtualPlugin } from "./compiler.ts";
import { sourcePlugin, mapSource } from "./source.ts";
import { prerender, type PrerenderOptions } from "./prerender/prerender.ts";
import { type DependencyResolution } from "@destack/package";
import { dependencyPlugin, type ModuleSource } from "./dependency.ts";
import { describeAssets, directoryPlugin } from "./asset.ts";
import { externalModule, runtimeConditions, checkRuntime } from "./runtime.ts";
import { type DirectoryReference } from "../typescript/index.ts";
import { type ModuleDescription } from "@destack/package/code";
import { type RuntimeCompiler } from "../compile/runtime.ts";
import { tmpdir } from "node:os";
import { linkDependencies } from "../source/dependency.ts";

/** Solid application options supplied to its standard Vite integration. */
export interface ApplicationOptions extends Pick<
    StartOptions,
    | "app"
    | "document"
    | "entryClient"
    | "entryServer"
    | "middleware"
    | "setup"
    | "renderMode"
    | "css"
> {
    /** Compile a browser application and its server handler. */
    kind: "web";
    /** Named view declared in destack.json, mutually exclusive with app. */
    view?: string;
    /** Server runtime and whether to distribute the handler. */
    ssr: false | { runtime: "bun" | "workerd"; emit?: boolean };
    /** Public URL prefix used by browser assets. */
    base?: string;
    /** Minify generated JavaScript. */
    minify?: boolean;
    /** Package-relative public asset directory; defaults to public. */
    publicDirectory?: string | false;
    /** Render these public routes during the build. */
    prerender?: PrerenderOptions;
}

/** Browser assets and an optional request handler from a Solid application. */
export interface ApplicationCompilation {
    /** Compiler descriptions keyed by output name. */
    inspections: Record<string, BuildDescription>;
    /** Named client and SSR outputs. */
    outputs: Record<string, PackageOutput>;
    /** Generated files keyed by build-relative path. */
    files: Map<string, Uint8Array<ArrayBuffer>>;
    /** Generated files copied out of the framework's temporary directory. */
    paths: string[];
    /** Maps emitted by the client and server compilers. */
    sourceMaps: SourceMapReference[];
}

/** Build a Solid application through the upstream Start environment orchestration. */
export async function compileApplication(
    name: string,
    start: ApplicationOptions,
    client: CompileOptions,
    server: CompileOptions | undefined,
    sources: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    project: PackageSource,
    dependencies: Readonly<Record<string, DependencyResolution>>,
    directories: ReadonlyMap<string, readonly DirectoryReference[]>,
    serverModules: readonly ModuleDescription[],
    runtimes: RuntimeCompiler,
    destinationRoot: string,
): Promise<ApplicationCompilation> {
    // stage the package and collect its application files
    await using stage = await stagePackage(project);
    const temporary = join(stage.directory, "dist");
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const paths: string[] = [];
    const sourceMaps: SourceMapReference[] = [];
    const locations = new Map<string, ModuleSource>();
    const assets = new Map<string, Uint8Array<ArrayBuffer>>();
    const outputs: Record<string, PackageOutput> = {};
    const inspections: Record<string, BuildDescription> = {
        client: { packages: {}, inputs: {}, outputs: {} },
        ssr: { packages: {}, inputs: {}, outputs: {} },
    };

    // let Start build the client before the server reads its asset manifest
    {
        const builder = await createBuilder({
            root: stage.directory,
            configFile: false,
            envDir: false,
            publicDir: start.publicDirectory ?? "public",
            logLevel: "silent",
            define: { "process.env.NODE_ENV": JSON.stringify("production") },
            base: client.base ?? "/",
            ssr: {
                noExternal: true,
                target: server?.runtime === "workerd" ? "webworker" : "node",
                resolve: {
                    conditions: runtimeConditions(server?.runtime ?? "bun"),
                },
            },
            plugins: [
                directoryPlugin(project.directory, files, directories, assets),
                virtualPlugin(stage.directory),
                sourcePlugin(project.directory, files, sources, destinationRoot, paths),
                dependencyPlugin(
                    project,
                    dependencies,
                    inspections.client,
                    `output/${name}-browser`,
                    "client",
                    locations,
                    assets,
                ),
                dependencyPlugin(
                    project,
                    dependencies,
                    inspections.ssr,
                    `output/${name}-server`,
                    "ssr",
                    locations,
                    assets,
                ),
                ...compilationPlugins(
                    project,
                    server !== undefined || start.prerender !== undefined,
                    start,
                ),
                {
                    name: "destack-output",
                    enforce: "post",
                    config() {
                        return {
                            environments: {
                                client: {
                                    build: {
                                        outDir: join(temporary, "client"),
                                        minify: client.minify ?? false,
                                        rolldownOptions: {
                                            resolve: {
                                                conditionNames: runtimeConditions("browser"),
                                            },
                                            external: (specifier) =>
                                                externalModule(specifier, "browser"),
                                        },
                                    },
                                },
                                ssr: {
                                    resolve: {
                                        conditions: runtimeConditions(server?.runtime ?? "bun"),
                                    },
                                    build: {
                                        outDir: join(temporary, "server"),
                                        emitAssets: true,
                                        copyPublicDir: false,
                                        minify: server?.minify ?? false,
                                        rolldownOptions: {
                                            resolve: {
                                                mainFields:
                                                    server?.runtime === "workerd"
                                                        ? ["browser", "module", "main"]
                                                        : ["module", "main"],
                                                conditionNames: runtimeConditions(
                                                    server?.runtime ?? "bun",
                                                ),
                                            },
                                            external: (specifier) =>
                                                externalModule(specifier, server?.runtime ?? "bun"),
                                            platform:
                                                server?.runtime === "workerd" ? "browser" : "node",
                                        },
                                    },
                                },
                            },
                        };
                    },
                },
            ],
            build: {
                sourcemap: true,
                copyPublicDir: true,
                rolldownOptions: {
                    cwd: project.directory,
                    experimental: { attachDebugInfo: "none" },
                    output: {
                        sourcemapPathTransform(source, map) {
                            const generated = relative(temporary, map)
                                .split(sep)
                                .join("/")
                                .replace(/^server\//, "ssr/");

                            return mapSource(
                                source,
                                map,
                                `output/${name}-${generated
                                    .replace("client/", "browser/")
                                    .replace("ssr/", "server/")}`,
                                locations,
                            );
                        },
                    },
                },
            },
        });
        await builder.buildApp();

        // check server entrypoints and dependency imports before prerendering shared views
        if (server) {
            const entrypoints = new Set(
                [start.entryServer, start.middleware]
                    .filter((path): path is string => !!path)
                    .map((path) =>
                        relative(project.directory, resolve(project.directory, path))
                            .split(sep)
                            .join("/"),
                    ),
            );
            const modules = serverModules.filter((module) => entrypoints.has(module.path));
            await checkRuntime(inspections.ssr, modules, server.runtime ?? "bun", runtimes);
        }

        // retain prerendered pages alongside browser assets for static or hybrid hosting
        if (start.prerender) {
            const pages = await prerender(join(temporary, "server/server.js"), start.prerender);
            for (const [path, bytes] of pages) {
                files.set(`output/${name}-browser/${path}`, bytes);
            }
        }

        // retain only deployable output directories, excluding the source checkout
        for (const [side, target] of [
            ["client", "browser"],
            ...(server ? [["ssr", server.target]] : []),
        ] as const) {
            const directory = `output/${name}-${side === "client" ? "browser" : "server"}`;
            const shouldDistribute =
                side === "client" || (start.ssr !== false && start.ssr.emit !== false);
            const entries = shouldDistribute
                ? await readdir(join(temporary, side === "ssr" ? "server" : side), {
                      recursive: true,
                      withFileTypes: true,
                  })
                : [];
            for (const entry of entries) {
                if (!entry.isFile()) {
                    continue;
                }
                const path = join(entry.parentPath, entry.name);
                const relative = path
                    .slice(join(temporary, side === "ssr" ? "server" : side).length + 1)
                    .replaceAll("\\", "/");
                const output = `${directory}/${relative}`;
                if (relative === ".vite/manifest.json") {
                    const bytes = new TextEncoder().encode(
                        JSON.stringify(
                            describeAssets(
                                JSON.parse(await readFile(path, "utf8")),
                                stage.directory,
                                locations,
                            ),
                        ),
                    );
                    files.set(output, bytes);
                } else {
                    const destination = join(destinationRoot, output);
                    await mkdir(dirname(destination), { recursive: true });
                    await copyFile(path, destination);
                    paths.push(output);
                }
                if (relative.endsWith(".js.map")) {
                    sourceMaps.push({
                        generated: `${directory}/${relative.slice(0, -4)}`,
                        map: `${directory}/${relative}`,
                    });
                }
            }
            outputs[side] = {
                target: target as "browser" | "server",
                runtime: side === "client" ? "browser" : (server?.runtime ?? "bun"),
                emit: true,
                ...(start.view === undefined ? {} : { view: start.view }),
                workloads: {},
                descriptions: {},
                directory,
                exports:
                    side === "ssr"
                        ? { ".": `${directory}/server.js` }
                        : server || start.prerender
                          ? {}
                          : { ".": `${directory}/index.html` },
                dependencies: {},
            };
        }

        return {
            outputs,
            files,
            paths,
            sourceMaps,
            inspections,
        };
    }
}

/** Isolate framework output while retaining normal package dependency resolution. */
async function stagePackage(
    source: PackageSource,
): Promise<AsyncDisposable & { directory: string }> {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-")));

    // link package inputs while reserving the framework's output directory
    try {
        for (const entry of await readdir(source.directory, { withFileTypes: true })) {
            if (
                entry.name === "dist" ||
                entry.name === "node_modules" ||
                entry.name === ".git" ||
                entry.name === "tsconfig.json"
            ) {
                continue;
            }
            await symlink(
                join(source.directory, entry.name),
                join(directory, entry.name),
                entry.isDirectory() ? "junction" : "file",
            );
        }

        // resolve dependencies through the nearest installed package tree
        await linkDependencies(source.directory, directory);

        // retain authored relative paths through the selected absolute compiler configuration
        await writeFile(
            join(directory, "tsconfig.json"),
            JSON.stringify({ extends: source.configuration }),
        );
    } catch (error) {
        await rm(directory, { recursive: true });
        throw error;
    }

    return {
        directory,
        async [Symbol.asyncDispose]() {
            await rm(directory, { recursive: true });
        },
    };
}
