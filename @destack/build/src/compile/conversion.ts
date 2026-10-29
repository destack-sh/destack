import type { FileSystem } from "@destack/environment";
import { isBuiltin } from "node:module";
import { dirname, join, relative, sep } from "node:path";
import * as lexer from "cjs-module-lexer";
import { type Plugin, rolldown } from "rolldown";
import { DependencyName } from "@destack/package";
import { HostModules, isHostModule, runtimeConditions } from "@destack/package/build";
import { present, schema } from "@destack/schema";
import { type Runtime, WORKERD_SCHEME } from "@destack/package/runtime";
import { BuildError } from "../error/index.ts";

/** The JavaScript module files a conversion turns into ESM, by extension. */
const MODULE_PATTERN = /\.[cm]?js$/u;

/** A file a framework compiles rather than JavaScript loads, such as a `.svelte` component. */
const FOREIGN_PATTERN = /\.(?:svelte|vue|astro|css|scss|sass|less|styl)$/u;

/** A bare specifier, which names another package or a runtime module and stays external. */
const BARE_PATTERN = /^[^./\0#]/u;

/** The directory a conversion writes rolldown's runtime helper modules into. */
const HELPER_DIRECTORY = "_rolldown";

/** The prefix of the module standing in for a require of a package or runtime module. */
const REQUIRE_PREFIX = "\0destack-require:";

/** The prefix of the module standing in for a require of an optional dependency left out. */
const ABSENT_PREFIX = "\0destack-absent:";

/** The module standing in for a host module a package's `browser` field replaces with nothing. */
const EMPTY_MODULE = "\0destack-empty";

/** The module export of a converted CommonJS module, as rolldown renders it. */
const CONVERTED_EXPORT = /^export default (require_[\w$]+)\(\);$/mu;

/** The names a converted module exports itself, which Node's lexer also reports. */
const IMPLIED_EXPORTS: ReadonlySet<string> = new Set(["default", "module.exports", "__esModule"]);

/** The suffixes Node tries for a relative require, in order. */
const RESOLVED_SUFFIXES = ["", ".js", ".cjs", ".json", "/index.js", "/index.cjs"];

/** The fields declaring a package's entry, as Wrangler and browser bundlers read them. */
const MAIN_FIELDS = ["browser", "module", "main"];

/** The package.json fields declaring a package's entries. */
const EntryManifest = schema.looseObject({
    exports: schema.unknown().exactOptional(),
    browser: schema.unknown().exactOptional(),
    module: schema.string().exactOptional(),
    main: schema.string().exactOptional(),
});

/** The runtimes an npm package converts for, the ones loading ES modules only. */
export type ConvertedRuntime = Exclude<Runtime, "bun">;

/** A module a conversion converted: its path and its code. */
interface ConvertedModule {
    /** The module's path in the package. */
    readonly path: string;
    /** The converted ESM code. */
    readonly code: string;
}

/** An npm package converted for a runtime: its files, its exports' modules and its imports. */
export interface ConvertedPackage {
    /** The installed files with every exported module converted, by package path. */
    readonly files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
    /** The module each export subpath resolves to. */
    readonly exports: Readonly<Record<string, string>>;
    /** The installed packages and peers its converted modules import. */
    readonly imported: ReadonlySet<string>;
}

/** The conversion of one npm package to ESM for workerd or the browser. */
export class NpmConversion {
    /** The package's name. */
    readonly name: string;
    /** The runtime the converted modules load on. */
    readonly runtime: ConvertedRuntime;
    /** The installed packages it requires, which stay external. */
    readonly dependencies: ReadonlySet<string>;
    /** The peers it declares, which the tree it sits in provides. */
    readonly peers: ReadonlySet<string>;
    /** The files the conversion reads and writes. */
    readonly #files: FileSystem;

    /** Store a package's name, dependencies and peers. */
    constructor(
        name: string,
        dependencies: readonly string[],
        peers: readonly string[],
        runtime: ConvertedRuntime,
        files: FileSystem,
    ) {
        // store the package's requirements, its runtime and the files it converts
        this.name = name;
        this.runtime = runtime;
        this.dependencies = new Set(dependencies);
        this.peers = new Set(peers);
        this.#files = files;
    }

    /** Convert an installed copy's exported modules, answering its files, exports and imports. */
    async convert(files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>): Promise<ConvertedPackage> {
        // copy the package where rolldown keys its modules plainly
        const root = await this.#files.realPath(
            await this.#files.makeTemporaryDirectory("destack-npm-"),
        );
        try {
            for (const [path, bytes] of files) {
                await this.#files.mkdir(dirname(join(root, path)), { recursive: true });
                await this.#files.writeFile(join(root, path), bytes);
            }

            // convert every module its exports import over the installed files
            const exports = this.#exports(files);
            const modules = [...new Set(exports.values())].toSorted();
            const imported = new Set<string>();
            const converted = new Map(files);
            if (modules.length > 0) {
                await lexer.init();
                for (const { path, code } of await this.#rolldown(root, modules, files, imported)) {
                    converted.set(path, new TextEncoder().encode(code));
                }
            }

            return { files: converted, exports: Object.fromEntries(exports), imported };
        } finally {
            await this.#files.remove(root, { recursive: true });
        }
    }

    /** Resolve a converted module's imports by Node's rules. */
    #plugin(
        root: string,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        seen: Set<string>,
    ): Plugin {
        // leave runtime modules, empty replaced ones, and take requires as Node's require(esm) does
        const hosts = new HostModules(this.runtime, this.#files);
        const { name, runtime } = this;
        const known = new Set([name, ...this.dependencies, ...this.peers]);

        return {
            name: `destack-npm-${runtime}`,
            resolveId: async (specifier, importer, options) => {
                // leave a relative import of a file JavaScript does not load, such as a component's source
                if (specifier.startsWith(".") && FOREIGN_PATTERN.test(specifier)) {
                    return { id: specifier, external: true };
                }
                // leave relative, package-internal and stand-in modules, and rolldown's helpers' imports, to rolldown
                else if (
                    importer === undefined ||
                    importer.startsWith("\0") ||
                    !BARE_PATTERN.test(specifier)
                ) {
                    return null;
                }
                const isRequire = options.kind === "require-call";
                const resolution =
                    runtime === "workerd" && WORKERD_SCHEME.test(specifier)
                        ? "external"
                        : await hosts.classify(specifier, importer);
                const imported = DependencyName.of(specifier);
                // leave a runtime module, and empty one a `browser` field replaces
                if (resolution === "external" || resolution === "empty") {
                    const required = isRequire ? `${REQUIRE_PREFIX}${specifier}` : null;

                    return resolution === "empty"
                        ? EMPTY_MODULE
                        : (required ?? { id: specifier, external: true });
                }
                // fail a require of what the runtime lacks when it runs, as Node fails a missing module
                else if ((resolution === "unavailable" || !known.has(imported)) && isRequire) {
                    return `${ABSENT_PREFIX}${specifier}`;
                }
                // leave an import of what the runtime lacks, which fails once the runtime loads it
                else if (resolution === "unavailable" || !known.has(imported)) {
                    return { id: specifier, external: true };
                }
                // mark an installed package or peer external, required through its stand-in
                else {
                    seen.add(imported);

                    return isRequire
                        ? `${REQUIRE_PREFIX}${specifier}`
                        : { id: specifier, external: true };
                }
            },
            load: (id) => {
                // load a stand-in as JavaScript, whatever its specifier's extension
                const code = NpmConversion.#standIn(id, runtime);

                return code === null ? null : { code, moduleType: "js" };
            },
            renderChunk(code, chunk) {
                // export a module rolldown read as CommonJS by its names
                const facade = chunk.facadeModuleId;
                const isCommonJS =
                    facade !== null && this.getModuleInfo(facade)?.inputFormat === "cjs";

                return NpmConversion.#exportConverted(code, facade, isCommonJS, root, files);
            },
        };
    }

    /** Write the module standing in for a require, an absent package or an emptied host module. */
    static #standIn(id: string, runtime: ConvertedRuntime): string | null {
        // require a runtime module by its default, and a package by its `module.exports` export
        if (id.startsWith(REQUIRE_PREFIX)) {
            const specifier = JSON.stringify(id.slice(REQUIRE_PREFIX.length));
            const exported = isBuiltin(id.slice(REQUIRE_PREFIX.length))
                ? "m.default"
                : 'Object.hasOwn(m, "module.exports") ? m["module.exports"] : m';

            return `import * as m from ${specifier};\nmodule.exports = ${exported};\n`;
        }
        // fail a require of what the runtime lacks when the require runs
        else if (id.startsWith(ABSENT_PREFIX)) {
            const specifier = id.slice(ABSENT_PREFIX.length);
            const message = JSON.stringify(
                isHostModule(specifier)
                    ? `host module is unavailable on ${runtime}: ${specifier}`
                    : `Cannot find module '${specifier}'`,
            );

            return `module.exports = {};\nthrow Object.assign(new Error(${message}), { code: "MODULE_NOT_FOUND" });\n`;
        }
        // export nothing for an emptied host module
        else if (id === EMPTY_MODULE) {
            return "module.exports = {};\n";
        }

        return null;
    }

    /** The conditions the package's imports resolve under, as Wrangler and bundlers read them. */
    get conditions(): string[] {
        return [...runtimeConditions(this.runtime), "import", "require", "default"];
    }

    /** Map each export subpath to the module it resolves to under the runtime's conditions. */
    #exports(files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>): Map<string, string> {
        // read the package's exports by subpath, else its first main field
        const manifest = EntryManifest.parse(
            JSON.parse(
                new TextDecoder().decode(present(files.get("package.json"), "package.json")),
            ),
        );
        const main = MAIN_FIELDS.map((field) => Reflect.get(manifest, field)).find(
            (value): value is string => typeof value === "string",
        );
        const subpaths =
            manifest.exports === undefined
                ? [[".", main ?? "index.js"] as const]
                : this.#subpaths(manifest.exports);

        // take each target's module, and every module a pattern matches under its subpath
        const modules = [...files.keys()].filter((path) => MODULE_PATTERN.test(path));
        const exported = new Map<string, string>();
        for (const [subpath, target] of subpaths) {
            // take a plain target's module
            const path = target.replace(/^\.\//u, "");
            if (!path.includes("*")) {
                const module = NpmConversion.#moduleAt(path, files);
                if (module !== undefined && MODULE_PATTERN.test(module)) {
                    exported.set(subpath, module);
                }
            }
            // take every module a pattern matches, under the subpath its match fills
            else {
                const [prefix = "", suffix = ""] = path.split("*");
                const [before = "", after = ""] = subpath.split("*");
                for (const module of modules) {
                    if (module.startsWith(prefix) && module.endsWith(suffix)) {
                        const match = module.slice(prefix.length, module.length - suffix.length);
                        exported.set(`${before}${match}${after}`, module);
                    }
                }
            }
        }

        return exported;
    }

    /** List an exports field's subpaths with the target each maps to. */
    #subpaths(exports: unknown): (readonly [string, string])[] {
        // read a subpath map by its keys, and any other form as the package's root
        const isSubpaths =
            typeof exports === "object" &&
            exports !== null &&
            !Array.isArray(exports) &&
            Object.keys(exports).some((key) => key.startsWith("."));
        const entries = isSubpaths ? Object.entries(exports) : [[".", exports] as const];

        return entries.flatMap(([subpath, value]) => {
            const [target] = this.#exportTargets(value);

            return target === undefined ? [] : [[subpath, target] as const];
        });
    }

    /** List the file an export's conditions map to under the runtime's conditions. */
    #exportTargets(exports: unknown): string[] {
        // take a path, the first alternative that maps, or the first condition the runtime matches
        if (typeof exports === "string") {
            return [exports];
        } else if (Array.isArray(exports)) {
            return exports.flatMap((each) => this.#exportTargets(each)).slice(0, 1);
        } else if (typeof exports !== "object" || exports === null) {
            return [];
        }
        const { conditions } = this;
        const matched = Object.entries(exports).find(([condition]) =>
            conditions.includes(condition),
        );

        return matched === undefined ? [] : this.#exportTargets(matched[1]);
    }

    /** Export a converted CommonJS module by default, by `module.exports` and by name. */
    static #exportConverted(
        code: string,
        id: string | null,
        isCommonJS: boolean,
        root: string,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ): string {
        // match rolldown's export of a converted module
        const match = CONVERTED_EXPORT.exec(code);

        // refuse a CommonJS module rendered without rolldown's wrapper
        if (match === null && isCommonJS) {
            throw BuildError.of("BUNDLE_FAILED", `unrecognized CommonJS export of ${id}`);
        }
        // leave an ES module
        else if (match === null) {
            return code;
        }
        // evaluate an absent dependency's stand-in only when required
        else if (id?.startsWith(ABSENT_PREFIX) === true) {
            return code.replace(CONVERTED_EXPORT, "");
        }

        // export the exports by default, or their default once transpiled from ESM, as bundlers do
        const required = match[1];
        const lines = [
            `const exported = ${required}();`,
            "const exportedDefault = exported?.__esModule === true ? exported.default : exported;",
            'export { exportedDefault as default, exported as "module.exports" };',
        ];

        // export each name Node's lexer finds, following the package's re-exports
        const path = id !== null && id.startsWith(`${root}${sep}`) ? relative(root, id) : undefined;
        const names =
            path === undefined
                ? []
                : NpmConversion.#namesOf(path.split(sep).join("/"), files, new Set());
        for (const [index, name] of names.entries()) {
            lines.push(`const named${index} = exported[${JSON.stringify(name)}];`);
            lines.push(`export { named${index} as ${JSON.stringify(name)} };`);
        }

        // replace with a function, as a wrapper name's `$` would read as a replacement pattern
        return code.replace(CONVERTED_EXPORT, () => lines.join("\n"));
    }

    /** List the names a CommonJS module exports as Node's lexer reads them, re-exports included. */
    static #namesOf(
        path: string,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        visited: Set<string>,
    ): string[] {
        // read each module once
        const source = files.get(path);
        if (source === undefined || visited.has(path)) {
            return [];
        }
        visited.add(path);

        // take the module's names, then the names of the package modules it re-exports
        const lexed = lexer.parse(new TextDecoder().decode(source));
        const names = new Set(lexed.exports.filter((name) => !IMPLIED_EXPORTS.has(name)));
        for (const reexport of lexed.reexports.filter((each) => each.startsWith("."))) {
            const target = NpmConversion.#moduleAt(
                join(dirname(path), reexport).split(sep).join("/"),
                files,
            );
            for (const name of target === undefined
                ? []
                : NpmConversion.#namesOf(target, files, visited)) {
                names.add(name);
            }
        }

        return [...names].toSorted();
    }

    /** Find the module a relative require names, trying Node's extensions and index files. */
    static #moduleAt(
        path: string,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ): string | undefined {
        return RESOLVED_SUFFIXES.map((suffix) => `${path}${suffix}`).find((each) =>
            files.has(each),
        );
    }

    /** Convert modules under a root to ESM, one file each. */
    async #rolldown(
        root: string,
        modules: readonly string[],
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        imported: Set<string>,
    ): Promise<ConvertedModule[]> {
        // resolve imports as the bundled form did, leaving other packages external
        const bundle = await rolldown({
            input: modules.map((path) => join(root, path)),
            cwd: root,
            // read no tsconfig a package publishes, whose extends name absent development packages
            tsconfig: false,
            // shim requires with Node's module on workerd, which has it, and inline them for the browser
            platform: this.runtime === "browser" ? "browser" : "node",
            logLevel: "silent",
            resolve: {
                conditionNames: this.conditions,
                mainFields: MAIN_FIELDS,
                aliasFields: [["browser"]],
            },
            plugins: [this.#plugin(root, files, imported)],
        });
        try {
            // key each module's file by its path, and rolldown's runtime helpers below their directory
            const named = (id: string | null, name: string) =>
                id !== null && id.startsWith(`${root}${sep}`)
                    ? relative(root, id)
                    : `${HELPER_DIRECTORY}/${name.replaceAll(/[^\w]/gu, "_")}.js`;
            const generated = await bundle.generate({
                format: "esm",
                preserveModules: true,
                entryFileNames: (chunk) => named(chunk.facadeModuleId, chunk.name),
                chunkFileNames: (chunk) => named(null, chunk.name),
            });

            // answer each module's code
            return generated.output.flatMap((chunk) =>
                chunk.type === "chunk" ? [{ path: chunk.fileName, code: chunk.code }] : [],
            );
        } finally {
            await bundle.close();
        }
    }
}
