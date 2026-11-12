import type { Crypto } from "@destack/environment";
import { canonicalize, Digest, type Version } from "@destack/schema";
import type { NpmPackage, PackageVersion, PackageRole } from "../definition/index.ts";
import type { Declaration } from "../graph/declaration.ts";
import { PackageError } from "../error/error.ts";
import { PackageFile } from "../file/file.ts";
import type { SourceMapReference } from "../source/map.ts";
import { CATALOG_DIRECTORY, MANIFEST_FORMAT, PackageManifest } from "./manifest.ts";
import { type OutputContent, PackageOutput } from "./output.ts";
import { MANIFEST_PATH } from "./reader.ts";

/** The path of each list a build's manifest references, inside the build. */
const MANIFEST_LISTS = {
    files: "manifest/files.json",
    sourceMaps: "manifest/sourceMaps.json",
    declarations: "manifest/declarations.json",
} as const;

/** The directory of each output's list of the files its runtime loads, inside the build. */
const OUTPUT_LISTS = "manifest/outputs";

/** The files a runtime never loads: source maps and type declarations. */
const UNLOADED_PATTERN = /\.map$|\.d\.[cm]?ts$/u;

/** The package manifests a build retains, which define its exports, runtimes and stamps. */
const DEFINITION_FILES: ReadonlySet<string> = new Set(["package.json", "destack.json"]);

/** The manifest dependents' compilations read, as its package.json has the build's version. */
const DEFINITION_FILE = "destack.json";

/** The media types of a build's files by extension, else octet streams (nginx's mime.types). */
const MEDIA_TYPES: Readonly<Record<string, string>> = {
    // documents and scripts
    html: "text/html",
    htm: "text/html",
    xhtml: "application/xhtml+xml",
    css: "text/css",
    js: "text/javascript",
    mjs: "text/javascript",
    cjs: "text/javascript",
    wasm: "application/wasm",

    // data and text
    json: "application/json",
    map: "application/json",
    jsonld: "application/ld+json",
    webmanifest: "application/manifest+json",
    xml: "application/xml",
    atom: "application/atom+xml",
    rss: "application/rss+xml",
    txt: "text/plain",
    md: "text/markdown",
    csv: "text/csv",
    ics: "text/calendar",
    vtt: "text/vtt",

    // images
    png: "image/png",
    apng: "image/apng",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    gif: "image/gif",
    webp: "image/webp",
    avif: "image/avif",
    svg: "image/svg+xml",
    ico: "image/vnd.microsoft.icon",
    bmp: "image/bmp",
    tif: "image/tiff",
    tiff: "image/tiff",

    // fonts
    woff: "font/woff",
    woff2: "font/woff2",
    ttf: "font/ttf",
    otf: "font/otf",

    // audio
    mp3: "audio/mpeg",
    m4a: "audio/mp4",
    aac: "audio/aac",
    wav: "audio/wav",
    ogg: "audio/ogg",
    oga: "audio/ogg",
    opus: "audio/opus",
    flac: "audio/flac",
    weba: "audio/webm",

    // video and streams
    mp4: "video/mp4",
    m4v: "video/mp4",
    webm: "video/webm",
    ogv: "video/ogg",
    mov: "video/quicktime",
    m3u8: "application/vnd.apple.mpegurl",

    // models
    glb: "model/gltf-binary",
    gltf: "model/gltf+json",

    // files people download
    pdf: "application/pdf",
    epub: "application/epub+zip",
    zip: "application/zip",
    gz: "application/gzip",
    tar: "application/x-tar",
    rtf: "application/rtf",
    docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    odt: "application/vnd.oasis.opendocument.text",
    ods: "application/vnd.oasis.opendocument.spreadsheet",
    odp: "application/vnd.oasis.opendocument.presentation",
};

/** Where a build writer keeps file bytes: a build directory, or memory. */
export interface BuildStorage {
    /** Keep a file's bytes at a package path. */
    readonly write: (path: string, bytes: Uint8Array<ArrayBuffer>) => Promise<void>;
}

/** What a build's manifest records beside its files. */
export interface BuildContent {
    /** The package being built: a Destack package, or an npm release built as a unit. */
    readonly package: PackageVersion | NpmPackage;
    /** What the package is, a library for an npm release. */
    readonly role: PackageRole;
    /** The Destack version whose toolchain builds it, and whose builtins it links. */
    readonly destack: Version;
    /** The named outputs, none when absent. */
    readonly outputs?: Readonly<Record<string, OutputContent>>;
    /** The packages it requires, by name, at their declared ranges, none when absent. */
    readonly requires?: Readonly<Record<string, string>>;
    /** The source maps of generated files, none when absent. */
    readonly sourceMaps?: readonly SourceMapReference[];
    /** The declarations the build's evaluation described, none when absent. */
    readonly declarations?: readonly Declaration[];
    /** Whether the package extends its dependents' builds, whose interface then has every file. */
    readonly isExtension?: boolean;
}

/** Write a build for `BuildReader`: its files, then its lists, then its manifest. */
export class BuildWriter {
    /** Where the bytes go. */
    readonly #storage: BuildStorage;
    /** The crypto digesting the written files. */
    readonly #crypto: Crypto;
    /** The written files, by package path. */
    readonly #files = new Map<string, PackageFile>();
    /** The writes of files' bytes under way, by package path. */
    readonly #writing = new Map<string, Promise<void>>();

    /** Write a build into storage, digesting its files with the crypto. */
    constructor(storage: BuildStorage, crypto: Crypto) {
        this.#storage = storage;
        this.#crypto = crypto;
    }

    /** Name a build file's media type by its extension. */
    static mediaType(path: string): string {
        return MEDIA_TYPES[path.slice(path.lastIndexOf(".") + 1)] ?? "application/octet-stream";
    }

    /** Write a file once, accepting a second write of equal bytes. */
    async write(path: string, bytes: Uint8Array<ArrayBuffer>): Promise<PackageFile> {
        // accept an equal file written before
        const file = await PackageFile.describe(
            path,
            BuildWriter.mediaType(path),
            bytes,
            this.#crypto,
        );
        if (this.#isWritten(file)) {
            await this.#writing.get(path);

            return file;
        }

        // record the file while its bytes are kept, so a concurrent equal write waits for them
        this.#files.set(path, file);
        const writing = this.#storage.write(path, bytes);
        this.#writing.set(path, writing);
        try {
            await writing;
        } catch (error) {
            // forget a file whose bytes were not kept
            this.#files.delete(path);
            throw error;
        } finally {
            this.#writing.delete(path);
        }

        return file;
    }

    /** Record a file in storage, such as compiler output, refusing other bytes at its path. */
    add(file: PackageFile): void {
        if (!this.#isWritten(file)) {
            this.#files.set(file.path, file);
        }
    }

    /** Read the description of a written file. */
    file(path: string): PackageFile {
        const file = this.#files.get(path);
        if (file === undefined) {
            throw PackageError.of(
                "FILE_INVALID",
                `build file '${path}' is invalid: it is unwritten`,
            );
        }

        return file;
    }

    /** Write the lists, then the manifest listing them, and return it. */
    async finish(contents: BuildContent): Promise<PackageManifest> {
        // write the lists, with the list of every file before them
        const files = [...this.#files.values()].toSorted((left, right) =>
            left.path < right.path ? -1 : Number(left.path > right.path),
        );
        const lists = {
            files: await this.#writeJson(MANIFEST_LISTS.files, files),
            sourceMaps: await this.#writeJson(MANIFEST_LISTS.sourceMaps, contents.sourceMaps ?? []),
            declarations: await this.#writeJson(
                MANIFEST_LISTS.declarations,
                contents.declarations ?? [],
            ),
        };

        // list the files each output's runtime loads
        const outputs: PackageManifest["outputs"] = {};
        for (const [name, output] of Object.entries(contents.outputs ?? {})) {
            const loaded = files.filter((file) =>
                BuildWriter.#isLoaded(file.path, output.directory),
            );
            const list = await this.#writeJson(`${OUTPUT_LISTS}/${name}.json`, loaded);
            outputs[name] = PackageOutput.parse({ ...output, files: list });
        }

        // digest what dependents' compilations read of the build
        const interfaceDigest = await Digest.json(
            {
                package: contents.package.name,
                definition: files.find((file) => file.path === DEFINITION_FILE)?.digest ?? null,
                requires: contents.requires ?? {},
                exports: Object.fromEntries(
                    Object.entries(outputs).map(([name, output]) => [
                        name,
                        {
                            runtime: output.runtime,
                            exports: Object.keys(output.exports).toSorted(),
                        },
                    ]),
                ),
                declarations: lists.declarations.digest,
                files: contents.isExtension === true ? lists.files.digest : null,
            },
            this.#crypto,
        );

        // write the manifest last, which makes the build readable
        const manifest: PackageManifest = {
            formatVersion: MANIFEST_FORMAT,
            package: contents.package,
            role: contents.role,
            destack: contents.destack,
            language: "typescript",
            requires: contents.requires ?? {},
            lists,
            interface: interfaceDigest,
            outputs,
        };
        await this.#storage.write(MANIFEST_PATH, PackageManifest.serialize(manifest));

        return manifest;
    }

    /** Report whether an output's runtime loads a file: its code, its definitions or a catalog. */
    static #isLoaded(path: string, directory: string): boolean {
        const isOwn = path.startsWith(`${directory}/`) && !UNLOADED_PATTERN.test(path);

        return isOwn || DEFINITION_FILES.has(path) || path.startsWith(`${CATALOG_DIRECTORY}/`);
    }

    /** Report whether a file was written with these bytes, refusing other bytes at its path. */
    #isWritten(file: PackageFile): boolean {
        const previous = this.#files.get(file.path);
        if (previous !== undefined && previous.digest !== file.digest) {
            throw PackageError.of(
                "FILE_INVALID",
                `build file '${file.path}' is invalid: it conflicts with another file at its path`,
            );
        }

        return previous !== undefined;
    }

    /** Write a description as canonical JSON, whose bytes depend on its value alone. */
    #writeJson(path: string, value: unknown): Promise<PackageFile> {
        return this.write(path, new TextEncoder().encode(canonicalize(value)));
    }
}
