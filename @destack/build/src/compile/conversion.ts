import { mkdir, realpath, writeFile } from "node:fs/promises";
import { isBuiltin } from "node:module";
import { dirname, join, relative, sep } from "node:path";
import * as lexer from "cjs-module-lexer";
import { type Plugin, rolldown } from "rolldown";
import { DependencyName } from "@destack/package";
import { HostModules, isHostModule, runtimeConditions } from "@destack/package/build";
import { present, schema } from "@destack/schema";
import { WORKERD_SCHEME } from "@destack/package/runtime";
import { NpmUnit } from "../dependency/npm.ts";
import { BuildError } from "../error/index.ts";

/** The JavaScript module files the workerd pass converts to ESM, by extension. */
const MODULE_PATTERN = /\.[cm]?js$/u;

/** A file a framework compiles rather than JavaScript loads, such as a `.svelte` component. */
const FOREIGN_PATTERN = /\.(?:svelte|vue|astro|css|scss|sass|less|styl)$/u;

/** A bare specifier, which names another package or a runtime module and stays external. */
const BARE_PATTERN = /^[^./\0#]/u;

/** The directory the workerd pass writes rolldown's runtime helper modules into. */
const HELPER_DIRECTORY = "_rolldown";

/** The prefix of the module standing in for a require of a package or runtime module. */
const REQUIRE_PREFIX = "\0destack-require:";

/** The prefix of the module standing in for a require of an optional dependency left out. */
const ABSENT_PREFIX = "\0destack-absent:";

/** The module standing in for a host module a package's `browser` field replaces with nothing. */
const EMPTY_MODULE = "\0destack-empty";

/** The module export of a converted CommonJS module, as rolldown renders it. */
const CONVERTED_EXPORT = /^export default (require_\w+)\(\);$/mu;

/** The names a converted module exports itself, which Node's lexer also reports. */
const IMPLIED_EXPORTS: ReadonlySet<string> = new Set(["default", "module.exports", "__esModule"]);

/** The suffixes Node tries for a relative require, in order. */
const RESOLVED_SUFFIXES = ["", ".js", ".cjs", ".json", "/index.js", "/index.cjs"];

/** The fields declaring a package's entry, as Wrangler reads them. */
const WORKERD_MAIN_FIELDS = ["browser", "module", "main"];

/** The package.json fields declaring a package's entries. */
const EntryManifest = schema.looseObject({
    exports: schema.unknown().exactOptional(),
    browser: schema.unknown().exactOptional(),
    module: schema.string().exactOptional(),
    main: schema.string().exactOptional(),
});

/** The conditions a workerd module's package-internal imports resolve under, as Wrangler's do. */
const WORKERD_CONDITIONS = [...runtimeConditions("workerd"), "import", "require", "default"];

/** A module the workerd pass converted: its path and its code. */
interface ConvertedModule {
    /** The module's path in the package. */
    readonly path: string;
    /** The converted ESM code. */
    readonly code: string;
}

/** The workerd conversion of one npm package to ESM in place, by Node's rules. */
export class NpmConversion {
    /** The package's name. */
    readonly name: string;
    /** The installed packages it requires, which stay external. */
    readonly dependencies: ReadonlySet<string>;
    /** The peers it declares, which the tree it sits in provides. */
    readonly peers: ReadonlySet<string>;

    /** Store a package's name, dependencies and peers. */
    constructor(name: string, dependencies: readonly string[], peers: readonly string[]) {
        this.name = name;
        this.dependencies = new Set(dependencies);
        this.peers = new Set(peers);
    }

    /** Convert an installed copy into a directory, answering the installed packages it imports. */
    async convert(source: string, destination: string): Promise<string[]> {
        // copy the package where rolldown keys its modules plainly
        const root = await realpath(destination);
        const files = await NpmUnit.files(source);
        for (const [path, bytes] of files) {
            await mkdir(dirname(join(root, path)), { recursive: true });
            await writeFile(join(root, path), bytes);
        }

        // convert every module its entries import, in place
        const modules = NpmConversion.#entries(files);
        const imported = new Set<string>();
        if (modules.length > 0) {
            await lexer.init();
            for (const { path, code } of await this.#rolldown(root, modules, files, imported)) {
                await mkdir(dirname(join(root, path)), { recursive: true });
                await writeFile(join(root, path), code);
            }
        }

        return [...imported].toSorted();
    }

    /** Resolve a converted module's imports by Node's rules, failing what workerd lacks at run. */
    #plugin(
        root: string,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        seen: Set<string>,
    ): Plugin {
        // leave runtime modules, empty replaced ones, and take requires as Node's require(esm) does
        const hosts = new HostModules("workerd");
        const name = this.name;
        const known = new Set([name, ...this.dependencies, ...this.peers]);

        return {
            name: "destack-npm-workerd",
            resolveId: (specifier, importer, options) => {
                // leave a relative import of a file JavaScript does not load, such as a component's source
                if (specifier.startsWith(".") && FOREIGN_PATTERN.test(specifier)) {
                    return { id: specifier, external: true };
                }
                // leave relative, package-internal and stand-in modules to rolldown
                else if (importer === undefined || !BARE_PATTERN.test(specifier)) {
                    return null;
                }
                const isRequire = options.kind === "require-call";
                const resolution = WORKERD_SCHEME.test(specifier)
                    ? "external"
                    : hosts.classify(specifier, importer);
                const imported = DependencyName.of(specifier);
                // leave a runtime module, and empty one a `browser` field replaces
                if (resolution === "external" || resolution === "empty") {
                    const required = isRequire ? `${REQUIRE_PREFIX}${specifier}` : null;

                    return resolution === "empty"
                        ? EMPTY_MODULE
                        : (required ?? { id: specifier, external: true });
                }
                // fail a require of what workerd lacks when it runs, as Node fails a missing module
                else if ((resolution === "unavailable" || !known.has(imported)) && isRequire) {
                    return `${ABSENT_PREFIX}${specifier}`;
                }
                // leave an import of what workerd lacks, which fails once workerd loads the module
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
                const code = NpmConversion.#standIn(id);

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
    static #standIn(id: string): string | null {
        // require a runtime module by its default, and a package by its `module.exports` export
        if (id.startsWith(REQUIRE_PREFIX)) {
            const specifier = JSON.stringify(id.slice(REQUIRE_PREFIX.length));
            const exported = isBuiltin(id.slice(REQUIRE_PREFIX.length))
                ? "m.default"
                : 'Object.hasOwn(m, "module.exports") ? m["module.exports"] : m';

            return `import * as m from ${specifier};\nmodule.exports = ${exported};\n`;
        }
        // fail a require of what workerd lacks when the require runs
        else if (id.startsWith(ABSENT_PREFIX)) {
            const specifier = id.slice(ABSENT_PREFIX.length);
            const message = JSON.stringify(
                isHostModule(specifier)
                    ? `host module is unavailable on workerd: ${specifier}`
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

    /** List the modules a package's entries resolve to under workerd's conditions. */
    static #entries(files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>): string[] {
        // read the package's exports, else its main fields
        const manifest = EntryManifest.parse(
            JSON.parse(
                new TextDecoder().decode(present(files.get("package.json"), "package.json")),
            ),
        );
        const targets =
            manifest.exports === undefined
                ? [
                      ...WORKERD_MAIN_FIELDS.flatMap((field) => {
                          const value = Reflect.get(manifest, field);

                          return typeof value === "string" ? [value] : [];
                      }),
                      "index.js",
                  ].slice(0, 1)
                : NpmConversion.#exportTargets(manifest.exports);

        // take each target's module, a pattern's every matching module
        const modules = [...files.keys()].filter((path) => MODULE_PATTERN.test(path));
        const entries = targets.flatMap((target) => {
            // take a plain target's module, or every module a pattern matches
            const path = target.replace(/^\.\//u, "");
            if (!path.includes("*")) {
                return [NpmConversion.#moduleAt(path, files)].filter((each) => each !== undefined);
            }
            const [prefix = "", suffix = ""] = path.split("*");

            return modules.filter((each) => each.startsWith(prefix) && each.endsWith(suffix));
        });

        return [...new Set(entries)].filter((path) => MODULE_PATTERN.test(path)).toSorted();
    }

    /** List the files an exports field maps to under workerd's conditions. */
    static #exportTargets(exports: unknown): string[] {
        // take a path, the first alternative that maps, or the first condition workerd matches
        if (typeof exports === "string") {
            return [exports];
        } else if (Array.isArray(exports)) {
            return exports.flatMap((each) => NpmConversion.#exportTargets(each)).slice(0, 1);
        } else if (typeof exports !== "object" || exports === null) {
            return [];
        }
        const entries = Object.entries(exports);
        const isSubpaths = entries.some(([key]) => key.startsWith("."));
        if (isSubpaths) {
            return entries.flatMap(([, target]) => NpmConversion.#exportTargets(target));
        }
        const matched = entries.find(([condition]) => WORKERD_CONDITIONS.includes(condition));

        return matched === undefined ? [] : NpmConversion.#exportTargets(matched[1]);
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
            throw new BuildError("BUILD_FAILED", `unrecognized CommonJS export of ${id}`);
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

        return code.replace(CONVERTED_EXPORT, lines.join("\n"));
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
            platform: "node",
            logLevel: "silent",
            resolve: {
                conditionNames: WORKERD_CONDITIONS,
                mainFields: WORKERD_MAIN_FIELDS,
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
