import { basename, dirname, isAbsolute, join, resolve } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import { Digest, type DigestCrypto } from "@destack/schema";
import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { type ESTree, Visitor } from "rolldown/utils";
import { type Plugin, type Manifest } from "vite";
import type { EmittedAsset } from "rolldown";
import { readDirectory } from "../build/directory.ts";
import { BuildError } from "../error/index.ts";
import { modulePackage, relativePath } from "../source/dependency.ts";
import { fileURLToPath, pathToFileURL } from "node:url";
import { type ModuleSource } from "./dependency.ts";
import { mapSource } from "./source.ts";

/** An imported file a build emits as an asset: a `?url` import, or a media or font file. */
const ASSET = /(?:\?url$|\.(?:png|jpe?g|gif|svg|webp|avif|ico|woff2?|ttf|otf|mp3|mp4|webm|wav)$)/u;

/** Emit each imported file as an asset whose URL the module exports by default, as Vite does. */
export function assetPlugin(files: FileSystem): Plugin {
    return {
        name: "@destack/build/asset",
        enforce: "pre",
        load: {
            filter: { id: ASSET },
            async handler(id) {
                // emit the file's bytes and export the URL its emitted copy has beside the module
                const [path = id] = id.split("?");
                const source = await files.readFile(path);
                const reference = this.emitFile({ type: "asset", name: basename(path), source });

                return `export default import.meta.ROLLUP_FILE_URL_${reference};\n`;
            },
        },
    };
}

/** Retain literal module-relative directories, including committed database migrations. */
export function directoryPlugin(
    directory: string,
    files: Map<string, Uint8Array<ArrayBuffer>>,
    sources: Map<string, Uint8Array<ArrayBuffer>>,
    environment: Environment<"files">,
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
            filter: { id: /\.[cm]?[jt]sx?$/u, code: /import\.meta\.url/u },
            async handler(code, id) {
                // read the module's literal directory URLs before other plugins transform the source
                const references = directoryReferences(code, id);
                if (references.length === 0) {
                    return;
                }
                const owner = await modulePackage(dirname(id), {}, environment.files);
                const rewritten = new MagicString(code);

                // bind each directory URL to a relocatable emitted asset
                for (const reference of references) {
                    const { source, local } = await resolveDirectory(
                        reference.path,
                        id,
                        owner.directory,
                        environment.files,
                    );

                    // emit the directory's files and add the package's to the build
                    const listing = readDirectory(source, environment.files);
                    const entries = await listing.catch((cause: unknown) => {
                        throw BuildError.of(
                            "ASSET_INVALID",
                            `${id} references an unreadable directory`,
                            { cause },
                        );
                    });
                    const url = await emitDirectory(
                        this,
                        reference.path,
                        source,
                        entries,
                        emitted,
                        environment.crypto,
                    );
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

/** A literal directory URL in a module: `new URL("./dir/", import.meta.url)`. */
interface DirectoryReference {
    /** The directory path relative to the module. */
    readonly path: string;
    /** The first character of the URL construction. */
    readonly start: number;
    /** The character after the URL construction. */
    readonly end: number;
}

/** List a module's literal directory URLs, none where the module binds its own `URL`. */
function directoryReferences(code: string, path: string): DirectoryReference[] {
    // leave a module importing or declaring its own URL constructor
    const program = parseAst(code, { lang: /\.[cm]?tsx$/u.test(path) ? "tsx" : "ts" }, path);
    const bindsUrl = program.body.some(
        (statement) =>
            (statement.type === "ImportDeclaration" &&
                statement.specifiers.some((specifier) => specifier.local.name === "URL")) ||
            ((statement.type === "ClassDeclaration" || statement.type === "FunctionDeclaration") &&
                statement.id?.name === "URL") ||
            (statement.type === "VariableDeclaration" &&
                statement.declarations.some(
                    (declaration) =>
                        declaration.id.type === "Identifier" && declaration.id.name === "URL",
                )),
    );
    if (bindsUrl) {
        return [];
    }

    // collect each construction of a relative directory against the module's URL
    const references: DirectoryReference[] = [];
    new Visitor({
        NewExpression(node) {
            const [location, base] = node.arguments;
            if (
                node.callee.type === "Identifier" &&
                node.callee.name === "URL" &&
                node.arguments.length === 2 &&
                isDirectoryLiteral(location) &&
                isModuleUrl(base)
            ) {
                references.push({ path: location.value, start: node.start, end: node.end });
            }
        },
    }).visit(program);

    return references;
}

/** Report whether an argument is a literal relative directory path, such as `./migration/`. */
function isDirectoryLiteral(
    node: ESTree.Argument | undefined,
): node is ESTree.StringLiteral & { value: string } {
    return (
        node?.type === "Literal" &&
        typeof node.value === "string" &&
        /^\.{1,2}\//u.test(node.value) &&
        node.value.endsWith("/")
    );
}

/** Report whether an argument is the expression `import.meta.url`. */
function isModuleUrl(node: ESTree.Argument | undefined): boolean {
    return (
        node?.type === "MemberExpression" &&
        !node.computed &&
        node.property.type === "Identifier" &&
        node.property.name === "url" &&
        node.object.type === "MetaProperty" &&
        node.object.meta.name === "import" &&
        node.object.property.name === "meta"
    );
}

/** Resolve a directory import inside its package to its real path and its package path. */
async function resolveDirectory(
    path: string,
    importer: string,
    packageDirectory: string,
    files: FileSystem,
): Promise<{ source: string; local: string }> {
    // follow symbolic links to the directory itself
    const source = await files.realPath(fileURLToPath(new URL(path, pathToFileURL(importer))));
    const local = relativePath(packageDirectory, source);
    if (local === ".." || local.startsWith("../") || isAbsolute(local)) {
        throw BuildError.of("ASSET_INVALID", `directory import is outside its package: ${path}`);
    }

    return { source, local };
}

/** Emit a directory's files under its digest and return the directory's URL expression. */
async function emitDirectory(
    context: { readonly emitFile: (file: EmittedAsset) => string },
    path: string,
    source: string,
    entries: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    emitted: Set<string>,
    crypto: DigestCrypto,
): Promise<string> {
    // key the directory by the digest of its file names and bytes
    const digest = await Digest.of(directoryBytes(entries), crypto);
    const destination = `asset/${digest}`;

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
        throw BuildError.of("ASSET_INVALID", `directory import is empty: ${path}`);
    }

    return `new URL(${JSON.stringify(anchor.parent)}, import.meta.ROLLUP_FILE_URL_${anchor.asset})`;
}

/** Add a directory's files as sources and at a package path in the build. */
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
        throw BuildError.of("INVARIANT_VIOLATED", `missing Vite manifest entry: ${key}`);
    }

    return value;
}

/** Stream each file's name and length, then its bytes, as a directory's digest reads them. */
async function* directoryBytes(
    entries: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
): AsyncIterable<Uint8Array> {
    // write each entry as JSON, then its bytes
    const encoder = new TextEncoder();
    for (const [name, bytes] of entries) {
        yield encoder.encode(JSON.stringify([name, bytes.length]));
        yield bytes;
    }
}
