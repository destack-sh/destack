import { createHash } from "node:crypto";
import { readdir, readFile, realpath } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, sep, resolve } from "node:path";
import MagicString from "magic-string";
import { type DirectoryReference } from "../inspect/directory.ts";
import { type Plugin, type Manifest } from "vite";
import { BuildError } from "../error/index.ts";
import { modulePackage } from "../source/dependency.ts";
import { fileURLToPath, pathToFileURL } from "node:url";
import { type ModuleSource } from "./dependency.ts";
import { mapSource } from "./source.ts";

/** Retain literal module-relative directories, including committed database migrations. */
export function directoryPlugin(
    directory: string,
    files: Map<string, Uint8Array<ArrayBuffer>>,
    directories: ReadonlyMap<string, readonly DirectoryReference[]>,
    sources: Map<string, Uint8Array<ArrayBuffer>>,
): Plugin {
    const emitted = new Set<string>();

    return {
        name: "destack-directory",
        enforce: "pre",
        resolveFileUrl({ referenceId, relativePath }) {
            if (!emitted.has(referenceId)) {
                return;
            }

            return `new URL(${JSON.stringify(relativePath)}, import.meta.url).href`;
        },
        transform: {
            filter: {
                id: [...directories.keys()].map(
                    (path) => new RegExp(`^${RegExp.escape(path.replaceAll("\\", "/"))}$`),
                ),
            },
            async handler(code, id) {
                // use compiler-resolved references before other plugins transform the source
                const references = directories.get(id);
                if (!references?.length) {
                    return;
                }
                const owner = await modulePackage(dirname(id));
                const rewritten = new MagicString(code);

                // preserve file names and bind the directory URL to a relocatable emitted asset
                for (const reference of references) {
                    const path = reference.path;
                    const source = await realpath(fileURLToPath(new URL(path, pathToFileURL(id))));
                    const local = relative(owner.directory, source).split(sep).join("/");
                    if (local === ".." || local.startsWith("../") || isAbsolute(local)) {
                        throw new BuildError(
                            "BUILD_FAILED",
                            `Directory import leaves its package: ${path}`,
                        );
                    }
                    const entries = await readDirectory(source);
                    if (!entries.size) {
                        throw new BuildError("BUILD_FAILED", `Directory import is empty: ${path}`);
                    }
                    const digest = createHash("sha256");
                    for (const [name, bytes] of entries) {
                        digest.update(JSON.stringify([name, bytes.length]));
                        digest.update(bytes);
                    }
                    const destination = `asset/${digest.digest("hex")}`;
                    let anchor: string | undefined;
                    let parent: string | undefined;
                    for (const [name, bytes] of entries) {
                        const asset = this.emitFile({
                            type: "asset",
                            fileName: `${destination}/${name}`,
                            originalFileName: join(source, name),
                            source: bytes,
                        });
                        emitted.add(asset);
                        sources.set(join(source, name), bytes);
                        if (anchor === undefined) {
                            anchor = asset;
                            parent = "../".repeat(name.split("/").length - 1) || "./";
                        }
                        if (owner.directory === directory) {
                            files.set(`${local}/${name}`, bytes);
                        }
                    }
                    rewritten.overwrite(
                        reference.start,
                        reference.end,
                        `new URL(${JSON.stringify(parent)}, import.meta.ROLLUP_FILE_URL_${anchor})`,
                    );
                }

                return {
                    code: rewritten.toString(),
                    map: rewritten.generateMap({ source: id, includeContent: true, hires: true }),
                };
            },
        },
    };
}

/** Read regular files in deterministic order and reject symbolic links. */
async function readDirectory(directory: string): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const pending = [""];
    for (const parent of pending) {
        const entries = await readdir(join(directory, parent), { withFileTypes: true });
        for (const entry of entries) {
            const name = parent ? `${parent}/${entry.name}` : entry.name;
            if (entry.isDirectory()) {
                pending.push(name);
            } else if (entry.isFile()) {
                files.set(name, new Uint8Array(await readFile(join(directory, name))));
            } else {
                throw new BuildError("BUILD_FAILED", `Unsupported directory entry: ${name}`);
            }
        }
    }

    return new Map(
        [...files].sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0)),
    );
}

/** Describe Vite assets without checkout-relative module identifiers. */
export function describeAssets(
    manifest: Manifest,
    directory: string,
    locations: ReadonlyMap<string, ModuleSource>,
): Manifest {
    // preserve generated chunk keys and qualify authored module keys
    const keys = new Map(
        Object.keys(manifest).map((key) => [
            key,
            key.startsWith("_")
                ? key
                : mapSource(key, resolve(directory, "manifest.json"), "manifest.json", locations),
        ]),
    );

    // update module references together with their entries
    return Object.fromEntries(
        Object.entries(manifest).map(([key, entry]) => [
            keys.get(key)!,
            {
                ...entry,
                ...(entry.src === undefined
                    ? {}
                    : {
                          src:
                              keys.get(entry.src) ??
                              mapSource(
                                  entry.src,
                                  resolve(directory, "manifest.json"),
                                  "manifest.json",
                                  locations,
                              ),
                      }),
                ...(entry.imports === undefined
                    ? {}
                    : {
                          imports: entry.imports.map((key) => assetKey(key, keys)),
                      }),
                ...(entry.dynamicImports === undefined
                    ? {}
                    : {
                          dynamicImports: entry.dynamicImports.map((key) => assetKey(key, keys)),
                      }),
            },
        ]),
    );
}

/** Resolve a compiler manifest import to its normalized entry. */
function assetKey(key: string, keys: ReadonlyMap<string, string>): string {
    const value = keys.get(key);
    if (value === undefined) {
        throw new BuildError("BUILD_FAILED", `Missing Vite manifest entry: ${key}`);
    }

    return value;
}
