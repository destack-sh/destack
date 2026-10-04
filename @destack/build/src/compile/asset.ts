import { createHash } from "node:crypto";
import { readdir, readFile, realpath } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import MagicString from "magic-string";
import { type DirectoryReference } from "../typescript/index.ts";
import { type Plugin, type Manifest } from "vite";
import type { EmittedAsset } from "rolldown";
import { BuildError } from "../error/index.ts";
import { modulePackage, relativePath } from "../source/dependency.ts";
import { fileURLToPath, pathToFileURL } from "node:url";
import { type ModuleSource } from "./dependency.ts";
import { mapSource } from "./source.ts";
import { compareText } from "../build/serialization.ts";

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
        resolveFileUrl({ referenceId, relativePath: path }) {
            if (!emitted.has(referenceId)) {
                return;
            }

            return `new URL(${JSON.stringify(path)}, import.meta.url).href`;
        },
        transform: {
            filter: {
                id: [...directories.keys()].map(
                    (path) => new RegExp(`^${RegExp.escape(path.replaceAll("\\", "/"))}$`, "u"),
                ),
            },
            async handler(code, id) {
                // use compiler-resolved references before other plugins transform the source
                const references = directories.get(id);
                if (references === undefined || references.length === 0) {
                    return;
                }
                const owner = await modulePackage(dirname(id));
                const rewritten = new MagicString(code);

                // bind each directory URL to a relocatable emitted asset
                for (const reference of references) {
                    const { source, local } = await resolveDirectory(
                        reference.path,
                        id,
                        owner.directory,
                    );

                    // emit the directory's files and keep the package's in the build
                    const entries = await readDirectory(source);
                    const url = await emitDirectory(this, reference.path, source, entries, emitted);
                    const path = owner.directory === directory ? local : undefined;
                    retainEntries(entries, source, path, files, sources);
                    rewritten.overwrite(reference.start, reference.end, url);
                }

                return {
                    code: rewritten.toString(),
                    map: rewritten.generateMap({ source: id, includeContent: true, hires: true }),
                };
            },
        },
    };
}

/** Resolve a directory import inside its package to its real path and its package path. */
async function resolveDirectory(
    path: string,
    importer: string,
    packageDirectory: string,
): Promise<{ source: string; local: string }> {
    // follow symbolic links to the directory itself
    const source = await realpath(fileURLToPath(new URL(path, pathToFileURL(importer))));
    const local = relativePath(packageDirectory, source);
    if (local === ".." || local.startsWith("../") || isAbsolute(local)) {
        throw new BuildError("BUILD_FAILED", `directory import leaves its package: ${path}`);
    }

    return { source, local };
}

/** Emit a directory's files under its digest and return the directory's URL expression. */
async function emitDirectory(
    context: { emitFile(file: EmittedAsset): string },
    path: string,
    source: string,
    entries: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    emitted: Set<string>,
): Promise<string> {
    // name the directory by the digest of its file names and bytes
    const digest = createHash("sha256");
    for (const [name, bytes] of entries) {
        digest.update(JSON.stringify([name, bytes.length]));
        digest.update(bytes);
    }
    const destination = `asset/${digest.digest("hex")}`;

    // emit each file, anchoring the directory at the first
    let anchor: { asset: string; parent: string } | undefined;
    for (const [name, bytes] of entries) {
        const asset = context.emitFile({
            type: "asset",
            fileName: `${destination}/${name}`,
            originalFileName: join(source, name),
            source: bytes,
        });
        emitted.add(asset);
        anchor ??= { asset, parent: "../".repeat(name.split("/").length - 1) || "./" };
    }
    if (anchor === undefined) {
        throw new BuildError("BUILD_FAILED", `directory import is empty: ${path}`);
    }

    return `new URL(${JSON.stringify(anchor.parent)}, import.meta.ROLLUP_FILE_URL_${anchor.asset})`;
}

/** Keep a directory's files as sources and keep them at a package path in the build. */
function retainEntries(
    entries: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    source: string,
    path: string | undefined,
    files: Map<string, Uint8Array<ArrayBuffer>>,
    sources: Map<string, Uint8Array<ArrayBuffer>>,
): void {
    for (const [name, bytes] of entries) {
        sources.set(join(source, name), bytes);
        if (path !== undefined) {
            files.set(`${path}/${name}`, bytes);
        }
    }
}

/** Read regular files in deterministic order and reject symbolic links. */
export async function readDirectory(
    directory: string,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // walk directories breadth first and collect regular files
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
                throw new BuildError("BUILD_FAILED", `unsupported directory entry: ${name}`);
            }
        }
    }

    return new Map([...files].toSorted(([left], [right]) => compareText(left, right)));
}

/** Describe Vite assets without checkout-relative module identifiers. */
export function describeAssets(
    manifest: Manifest,
    directory: string,
    locations: ReadonlyMap<string, ModuleSource>,
): Manifest {
    // preserve generated chunk keys and qualify authored module keys
    const map = resolve(directory, "manifest.json");
    const qualify = (key: string) => mapSource(key, map, "manifest.json", locations);
    const keys = new Map(
        Object.keys(manifest).map((key) => [key, key.startsWith("_") ? key : qualify(key)]),
    );
    const rename = (imported: string) => assetKey(imported, keys);

    // update module references together with their entries
    return Object.fromEntries(
        Object.entries(manifest).map(([key, entry]) => {
            const { src, imports, dynamicImports } = entry;
            const described = {
                ...entry,
                ...(src === undefined ? {} : { src: keys.get(src) ?? qualify(src) }),
                ...(imports === undefined ? {} : { imports: imports.map(rename) }),
                ...(dynamicImports === undefined
                    ? {}
                    : { dynamicImports: dynamicImports.map(rename) }),
            };

            return [assetKey(key, keys), described];
        }),
    );
}

/** Resolve a compiler manifest import to its normalized entry. */
function assetKey(key: string, keys: ReadonlyMap<string, string>): string {
    const value = keys.get(key);
    if (value === undefined) {
        throw new BuildError("BUILD_FAILED", `missing Vite manifest entry: ${key}`);
    }

    return value;
}
