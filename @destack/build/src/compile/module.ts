import { readFile } from "node:fs/promises";
import { relative, resolve, sep } from "node:path";
import { build } from "vite";
import { DependencyRelease, type DependencyResolution } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { type PackageOutput } from "@destack/package/manifest";
import { type BuildDescription } from "@destack/package/inspect";
import { type SourceMapReference } from "@destack/package/source";
import { type PackageSource } from "../source/index.ts";
import { BuildError } from "../error/index.ts";
import { compilationPlugins } from "./compiler.ts";
import { sourcePlugin, mapSource } from "./source.ts";
import { dependencyPlugin, type ModuleSource } from "./dependency.ts";
import { externalModule, runtimeConditions } from "./runtime.ts";
import { directoryPlugin } from "./asset.ts";
import { type DirectoryReference } from "../typescript/index.ts";
import { type Target } from "@destack/package";
import { type Runtime } from "@destack/package/runtime";

/** Compilation settings for a named package output. */
export interface CompileOptions {
    /** Compile package modules. */
    kind: "module";
    /** The execution target. */
    target: Target;
    /** The compiler runtime; browser targets use browser and server targets default to Bun. */
    runtime?: Runtime;
    /** Package-relative module or HTML entries; omit to compile package exports. */
    entries?: Readonly<Record<string, string>>;
    /** Bundle dependencies; disabled for separately installed library dependencies. */
    bundle?: boolean;
    /** Minify generated code with Vite's minifier. */
    minify?: boolean;
    /** Compile Solid components for server rendering and browser hydration. */
    ssr?: boolean;
    /** Public URL prefix used by browser assets. */
    base?: string;
}

/** Compiled output and its generated files. */
export interface Compilation {
    /** Dependencies, module imports, and generated file membership. */
    inspection: BuildDescription;
    /** The target output description. */
    output: PackageOutput;
    /** Generated files and captured source assets. */
    files: Map<string, Uint8Array<ArrayBuffer>>;
    /** Generated output paths written by Vite. */
    paths: string[];
    /** Maps for generated JavaScript. */
    sourceMaps: SourceMapReference[];
}

/** Compile one output using Vite, Solid, and StyleX. */
export async function compilePackage(
    name: string,
    options: CompileOptions,
    dependencies: Readonly<Record<string, DependencyResolution>>,
    sources: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    project: PackageSource,
    directories: ReadonlyMap<string, readonly DirectoryReference[]>,
    destinationRoot: string,
): Promise<Compilation> {
    // collect output paths and files under the output directory
    const directory = `output/${name}`;
    const destination = resolve(destinationRoot, directory);
    const paths: string[] = [];
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const exports: Record<string, string> = {};
    const external: Record<string, DependencyRelease> = {};
    const inspection: BuildDescription = { packages: {}, inputs: {}, outputs: {} };
    const locations = new Map<string, ModuleSource>();
    const assets = new Map<string, Uint8Array<ArrayBuffer>>();
    const sourceMaps: SourceMapReference[] = [];
    const isBrowser = options.target === "browser";
    const entries = Object.values(project.exports);
    if (!entries.length) {
        return {
            inspection,
            files,
            paths,
            sourceMaps,
            output: {
                target: options.target,
                runtime: project.runtime,
                emit: true,
                directory,
                workloads: {},
                descriptions: {},
                exports: {},
                dependencies: {},
            },
        };
    }
    if (entries.some((entry) => entry.endsWith(".html")) && !isBrowser) {
        throw new BuildError("BUILD_FAILED", "HTML entries require the browser target.");
    }

    // retain HTML before Vite rewrites its module and asset references
    for (const entry of entries.filter((entry) => entry.endsWith(".html"))) {
        files.set(
            relative(project.directory, entry).split(sep).join("/"),
            new Uint8Array(await readFile(entry)),
        );
    }

    // delegate JSX, CSS, assets, and minification to the standard plugins
    const generated = await build({
        root: project.directory,
        configFile: false,
        envDir: false,
        publicDir: false,
        base: options.base ?? "./",
        logLevel: "silent",
        define: { "process.env.NODE_ENV": JSON.stringify("production") },
        resolve: {
            conditions: runtimeConditions(project.runtime),
        },
        ssr: {
            noExternal: true,
            target: project.runtime === "workerd" ? "webworker" : "node",
            resolve: { conditions: runtimeConditions(project.runtime) },
        },
        plugins: [
            directoryPlugin(project.directory, files, directories, assets),
            sourcePlugin(project.directory, files, sources, destinationRoot, paths),
            dependencyPlugin(
                project,
                dependencies,
                inspection,
                directory,
                undefined,
                locations,
                assets,
            ),
            ...compilationPlugins(project, options.ssr === true),
        ],
        build: {
            emitAssets: true,
            write: true,
            outDir: destination,
            emptyOutDir: false,
            ssr: !isBrowser,
            minify: options.minify ?? false,
            sourcemap: true,
            copyPublicDir: false,
            rolldownOptions: {
                platform: project.runtime === "bun" ? "node" : "browser",
                resolve: {
                    mainFields:
                        project.runtime === "bun"
                            ? ["module", "main"]
                            : ["browser", "module", "main"],
                    conditionNames: runtimeConditions(project.runtime),
                },
                cwd: project.directory,
                experimental: { attachDebugInfo: "none" },
                input: entries,
                preserveEntrySignatures: "strict",
                onwarn(warning) {
                    throw new BuildError("BUILD_FAILED", warning.message);
                },
                external(specifier, importer) {
                    // reject host imports before Vite can emit browser stubs
                    if (externalModule(specifier, project.runtime)) {
                        return true;
                    }
                    if (
                        specifier.startsWith(".") ||
                        specifier.startsWith("/") ||
                        specifier.startsWith("\0") ||
                        specifier.startsWith("#")
                    ) {
                        return false;
                    }
                    const dependency = specifier.startsWith("@")
                        ? specifier.split("/").slice(0, 2).join("/")
                        : specifier.split("/")[0];
                    if (dependency === project.declaration.package.name) {
                        return false;
                    }
                    if (
                        options.bundle &&
                        importer &&
                        (importer.split(sep).includes("node_modules") ||
                            relative(project.directory, importer).startsWith(`..${sep}`))
                    ) {
                        return false;
                    }
                    if (
                        ![
                            project.declaration.dependencies,
                            project.declaration.peerDependencies,
                            project.declaration.optionalDependencies,
                        ].some((requirements) => Object.hasOwn(requirements, dependency))
                    ) {
                        throw new BuildError(
                            "BUILD_FAILED",
                            `Undeclared runtime dependency: ${dependency}`,
                        );
                    }
                    if (options.bundle) {
                        return false;
                    }
                    const selected = dependencies[dependency];
                    if (!selected) {
                        throw new BuildError(
                            "BUILD_FAILED",
                            `Unresolved runtime dependency: ${dependency}`,
                        );
                    }
                    if (selected.kind === "source") {
                        throw new BuildError(
                            "BUILD_FAILED",
                            `Runtime dependency requires a registry release: ${dependency}`,
                        );
                    }
                    external[dependency] = DependencyRelease.parse(selected);

                    return true;
                },
                output: {
                    format: "esm",
                    entryFileNames: "[name]-[hash].js",
                    chunkFileNames: "[name]-[hash].js",
                    assetFileNames: "asset/[name]-[hash][extname]",
                    sourcemapPathTransform(source, map) {
                        const output = `${directory}/${relative(destination, map).split(sep).join("/")}`;

                        return mapSource(source, map, output, locations);
                    },
                },
            },
        },
    });

    // retain generated paths and map authored entries to generated files
    if (!("output" in generated)) {
        throw new BuildError("BUILD_FAILED", "Expected one Vite output.");
    }
    for (const entry of generated.output) {
        const path = PackagePath.parse(`${directory}/${entry.fileName}`);
        paths.push(path);
        for (const [name, source] of Object.entries(project.exports)) {
            if (entry.type === "chunk" && entry.facadeModuleId === source) {
                exports[name] = path;
            }
            if (
                entry.type === "asset" &&
                source.endsWith(".html") &&
                entry.fileName === relative(project.directory, source).split(sep).join("/")
            ) {
                exports[name] = path;
            }
        }
        if (entry.type === "chunk" && entry.sourcemapFileName) {
            sourceMaps.push({ generated: path, map: `${directory}/${entry.sourcemapFileName}` });
        }
    }
    for (const name of Object.keys(project.exports)) {
        if (!exports[name]) {
            throw new BuildError("BUILD_FAILED", `Missing generated entry: ${name}`);
        }
    }

    return {
        inspection,
        output: {
            target: options.target,
            runtime: project.runtime,
            emit: true,
            workloads: {},
            descriptions: {},
            directory,
            exports,
            dependencies: external,
        },
        files,
        paths,
        sourceMaps,
    };
}
