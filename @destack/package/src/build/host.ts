import { type FileSystem, FileSystemError } from "@destack/environment";
import { dirname, join } from "node:path";
import type { Plugin } from "vite";
import { schema } from "@destack/schema";
import type { Runtime } from "../runtime/index.ts";
import { isHostModule, isRuntimeModule } from "./runtime.ts";
import { PackageError } from "../error/index.ts";

/** The module standing in for a host module a package's `browser` field replaces with nothing. */
const EMPTY_MODULE = "\0virtual:destack/empty";

/** The `browser` field of a package manifest: a replacement entry, or replacements by module. */
const BrowserField = schema.looseObject({
    browser: schema
        .union([
            schema.string(),
            schema.record(schema.string(), schema.union([schema.string(), schema.literal(false)])),
        ])
        .exactOptional(),
});

/** How a compilation takes a host module: kept external, or bundled as an empty module. */
export type HostResolution = "external" | "empty";

/** The host modules one runtime supplies, and those the importing packages replace with nothing. */
export class HostModules {
    /** The runtime the compilation targets. */
    readonly runtime: Runtime;
    /** The host modules each package's `browser` field replaces with nothing, by directory. */
    readonly #replaced = new Map<string, Promise<ReadonlySet<string>>>();
    /** The files the package manifests are read from. */
    readonly #files: FileSystem;

    /** Describe the host modules of one runtime, reading manifests from files. */
    constructor(runtime: Runtime, files: FileSystem) {
        this.runtime = runtime;
        this.#files = files;
    }

    /** Resolve a host module, refusing one the runtime lacks and the importing package keeps. */
    async resolve(
        specifier: string,
        importer: string | undefined,
    ): Promise<HostResolution | undefined> {
        // refuse the host modules the runtime lacks before Vite emits browser stubs
        const resolution = await this.classify(specifier, importer);
        if (resolution === "unavailable") {
            const importedBy = importer === undefined ? "" : `, imported by ${importer}`;
            throw PackageError.of(
                "HOST_MODULE_UNAVAILABLE",
                `host module is unavailable on ${this.runtime}: ${specifier}${importedBy}`,
            );
        }

        return resolution;
    }

    /** Classify a host module: kept, replaced with nothing, or unavailable; nothing for others. */
    async classify(
        specifier: string,
        importer: string | undefined,
    ): Promise<HostResolution | "unavailable" | undefined> {
        // keep host modules the runtime supplies
        if (isRuntimeModule(specifier, this.runtime)) {
            return "external";
        }
        // take the empty module for a host module the importing package replaces with nothing
        else if (await this.#isReplaced(specifier, importer)) {
            return "empty";
        }
        // mark the other host modules unavailable
        else if (isHostModule(specifier)) {
            return "unavailable";
        }

        return undefined;
    }

    /** Resolve host modules as the build takes them: kept external, empty, or refused. */
    plugin(): Plugin {
        return {
            name: "@destack/package/host",
            enforce: "pre",
            resolveId: async (specifier, importer) => {
                const resolution = await this.resolve(specifier, importer);

                // keep a supplied host module external
                if (resolution === "external") {
                    return { id: specifier, external: true };
                }
                // take the empty module for a replaced one
                else if (resolution === "empty") {
                    return EMPTY_MODULE;
                }
                // leave every other module to the build
                else {
                    return null;
                }
            },
            load: (id) => (id === EMPTY_MODULE ? "export default {};\n" : null),
        };
    }

    /** Report whether the importer's `browser` field replaces a host module with nothing. */
    async #isReplaced(specifier: string, importer: string | undefined): Promise<boolean> {
        // replace nothing on Bun, which supplies every host module, or for an entry
        if (this.runtime === "bun" || importer === undefined || !isHostModule(specifier)) {
            return false;
        }

        // read the nearest manifest's replacements once per directory
        const directory = dirname(importer.replace(/\?[\s\S]*$/u, ""));
        let replaced = this.#replaced.get(directory);
        if (replaced === undefined) {
            replaced = readReplaced(directory, this.#files);
            this.#replaced.set(directory, replaced);
        }

        return (await replaced).has(specifier);
    }
}

/** Read the host modules the nearest package manifest's `browser` field replaces with nothing. */
async function readReplaced(directory: string, files: FileSystem): Promise<ReadonlySet<string>> {
    for (let current = directory; ; current = dirname(current)) {
        // list the modules the nearest manifest's replacements map to false
        const text = await files
            .readTextFile(join(current, "package.json"))
            .catch(FileSystemError.absent);
        if (text !== undefined) {
            const { browser } = BrowserField.parse(JSON.parse(text));
            const replacements = typeof browser === "object" ? Object.entries(browser) : [];

            return new Set(replacements.filter(([, to]) => to === false).map(([from]) => from));
        }
        // replace nothing without a manifest up to the file system root
        else if (dirname(current) === current) {
            return new Set();
        }
    }
}
