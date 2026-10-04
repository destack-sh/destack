import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { Plugin } from "vite";
import { schema } from "@destack/schema";
import type { Runtime } from "../runtime/index.ts";
import { isHostModule, isRuntimeModule } from "./runtime.ts";

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
    /** The host modules each package's `browser` field replaces with nothing, by module directory. */
    readonly #replaced = new Map<string, ReadonlySet<string>>();

    /** Hold the host modules of one runtime. */
    constructor(runtime: Runtime) {
        this.runtime = runtime;
    }

    /** Resolve a host module, refusing one the runtime lacks and the importing package keeps. */
    resolve(specifier: string, importer: string | undefined): HostResolution | undefined {
        // keep host modules the runtime supplies
        if (isRuntimeModule(specifier, this.runtime)) {
            return "external";
        }
        // bundle the empty module for a host module the importing package replaces with nothing
        else if (this.#isReplaced(specifier, importer)) {
            return "empty";
        }
        // refuse the other host modules before Vite emits browser stubs
        else if (isHostModule(specifier)) {
            throw new TypeError(`host module is unavailable on ${this.runtime}: ${specifier}`);
        }

        return undefined;
    }

    /** Resolve the host modules the importing packages replace with nothing to an empty module. */
    plugin(): Plugin {
        return {
            name: "@destack/package/host",
            enforce: "pre",
            resolveId: (specifier, importer) =>
                this.#isReplaced(specifier, importer) ? EMPTY_MODULE : null,
            load: (id) => (id === EMPTY_MODULE ? "export default {};\n" : null),
        };
    }

    /** Report whether the importing package replaces a host module with nothing in its `browser` field. */
    #isReplaced(specifier: string, importer: string | undefined): boolean {
        // replace nothing on Bun, which supplies every host module, or for an entry
        if (this.runtime === "bun" || importer === undefined || !isHostModule(specifier)) {
            return false;
        }

        // read the nearest manifest's replacements once per directory
        const directory = dirname(importer.replace(/\?[\s\S]*$/u, ""));
        let replaced = this.#replaced.get(directory);
        if (replaced === undefined) {
            replaced = readReplaced(directory);
            this.#replaced.set(directory, replaced);
        }

        return replaced.has(specifier);
    }
}

/** Read the host modules the nearest package manifest's `browser` field replaces with nothing. */
function readReplaced(directory: string): ReadonlySet<string> {
    for (let current = directory; ; current = dirname(current)) {
        // list the modules the nearest manifest's replacements map to false
        const text = readManifest(join(current, "package.json"));
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

/** Read a package manifest, absent where none exists. */
function readManifest(path: string): string | undefined {
    try {
        return readFileSync(path, "utf8");
    } catch (error) {
        if (error instanceof Error && "code" in error && error.code === "ENOENT") {
            return undefined;
        }
        throw error;
    }
}
