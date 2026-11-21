import type { Environment, FileSystem } from "@destack/environment";
import type { PackageFile } from "../file/index.ts";
import { PackageError } from "../error/index.ts";
import { BuildImage } from "./image.ts";
import { PackageManifest } from "./manifest.ts";
import { BuildReader, MANIFEST_PATH } from "./reader.ts";
import { Digest, present, schema } from "@destack/schema";

/** The file of a layer directory indexing its releases, builds and files. */
export const LAYER_INDEX = "index.json";

/** The file of a layer directory listing its files as one table sorted by digest. */
export const LAYER_FILES = "files.bin";

/** The file of a layer directory listing the files each build distributes, sorted by build. */
export const LAYER_REFERENCES = "references.bin";

/** How a layer's file table finds a file: by its recorded path, or by its offset in the packs. */
const LAYER_ADDRESSINGS = ["path", "pack"] as const;

/** How a layer's file table finds a file. */
export type BuildLayerAddressing = (typeof LAYER_ADDRESSINGS)[number];

/** The layer index format a store writes. */
export const LAYER_FORMAT = 9;

/** The bytes of a file table's header: the row count, a little-endian u32. */
const FILES_HEADER = 4;

/** The bytes of a SHA-256 digest, the first field of a file table's row. */
const DIGEST_BYTES = 32;

/** The bytes of a file table's row: the digest, the byte count and the path's end or offset. */
const FILE_ROW = DIGEST_BYTES + 8;

/** The bytes of a reference row: the build's digest, the file's digest and its byte count. */
const REFERENCE_ROW = 2 * DIGEST_BYTES + 4;

/** The most bytes a file table's row records for one file, a u32's largest value. */
const MAXIMUM_FILE_BYTES = 0xff_ff_ff_ff;

/** The directory of a layer with its releases' dependency trees, laid out once on writing. */
export const LAYER_TREES = "trees";

/** The directory of a packed layer's manifests, each named by its digest. */
export const LAYER_BLOBS = "blobs";

/** The directory of a packed layer's packs, each named by its number. */
export const LAYER_PACKS = "packs";

/** The most bytes of packs a layer keeps read: 64 MiB, half of a Worker isolate's 128 MB. */
const KEPT_PACK_BYTES = 64 * 1024 * 1024;

/** The index of a layer directory, written once with it, so opening it reads one file. */
export const BuildLayerIndex = schema.object({
    /** The layer index format version. */
    formatVersion: schema.literal(LAYER_FORMAT),
    /** The images of the releases the layer ships, each with what it runs. */
    releases: schema.array(Digest),
    /** Each shipped image, by its digest. */
    images: schema.record(Digest, BuildImage),
    /** The byte count of each layered build's manifest, by the manifest's digest. */
    builds: schema.record(Digest, schema.int()),
    /** How the file table finds a file: by its recorded path, or by its offset in the packs. */
    addressing: schema.enum(LAYER_ADDRESSINGS),
    /** The byte count of each pack in order, none for a layer finding files by path. */
    packs: schema.array(schema.int()),
});
/** The index of a layer directory. */
export type BuildLayerIndex = schema.Infer<typeof BuildLayerIndex>;

/** A layered file, found at a path below the layer or at an offset in its packs. */
export type BuildLayerFile =
    | {
          /** The file's digest. */
          readonly digest: Digest;
          /** The file's byte count. */
          readonly size: number;
          /** A file of its own. */
          readonly addressing: "path";
          /** The file's path below the layer. */
          readonly path: string;
      }
    | {
          /** The file's digest. */
          readonly digest: Digest;
          /** The file's byte count. */
          readonly size: number;
          /** A file inside a pack. */
          readonly addressing: "pack";
          /** The file's offset in the packs laid end to end. */
          readonly offset: number;
      };

/** A layer's files as one table sorted by digest, searched in place as git's pack indexes are. */
export class BuildLayerTable {
    /** The table: the row count, the rows, then the paths' UTF-8 bytes. */
    readonly #bytes: Uint8Array;
    /** A view reading the table's u32 fields. */
    readonly #view: DataView;
    /** The number of rows. */
    readonly #count: number;
    /** How the table finds a file: by its recorded path, or by its offset in the packs. */
    readonly #addressing: BuildLayerAddressing;

    /** Read a table a layer wrote. */
    constructor(bytes: Uint8Array, addressing: BuildLayerAddressing) {
        // read the row count from the header, none for an empty table
        this.#bytes = bytes;
        this.#view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        this.#count = bytes.byteLength === 0 ? 0 : this.#view.getUint32(0, true);
        this.#addressing = addressing;
    }

    /** Write a table of files sorted by digest: each row's offset, or its path after the rows. */
    static encode(files: readonly BuildLayerFile[]): Uint8Array<ArrayBuffer> {
        // order the rows by digest, whose hexadecimal order is its bytes' order
        const sorted = files.toSorted((left, right) => (left.digest < right.digest ? -1 : 1));
        const encoder = new TextEncoder();
        const paths = sorted.map((file) =>
            file.addressing === "path" ? encoder.encode(file.path) : new Uint8Array(),
        );
        const pathBytes = paths.reduce((total, path) => total + path.byteLength, 0);

        // write the count, then each row, then the paths in row order
        const bytes = new Uint8Array(FILES_HEADER + sorted.length * FILE_ROW + pathBytes);
        const view = new DataView(bytes.buffer);
        view.setUint32(0, sorted.length, true);
        let end = 0;
        for (const [index, file] of sorted.entries()) {
            // read where the row points: a packed file's offset, else where its path ends
            const path = present(paths[index], `the path of row ${String(index)}`);
            end += path.byteLength;
            const pointer = file.addressing === "pack" ? file.offset : end;

            // refuse a byte count or an offset a row cannot hold
            if (file.size > MAXIMUM_FILE_BYTES || pointer > MAXIMUM_FILE_BYTES) {
                throw PackageError.of(
                    "LAYER_FILE_BROKEN",
                    `the layer cannot list file '${file.digest}': its '${String(file.size)}' bytes at '${String(pointer)}' exceed a row's u32`,
                );
            }

            // write the digest, the byte count, the pointer and the path
            const row = FILES_HEADER + index * FILE_ROW;
            bytes.set(Uint8Array.fromHex(file.digest), row);
            view.setUint32(row + DIGEST_BYTES, file.size, true);
            view.setUint32(row + DIGEST_BYTES + 4, pointer, true);
            bytes.set(path, FILES_HEADER + sorted.length * FILE_ROW + end - path.byteLength);
        }

        return bytes;
    }

    /** Find a file by its digest, absent for a file outside the table. */
    find(digest: Digest): BuildLayerFile | undefined {
        // search the sorted digests
        const wanted = Uint8Array.fromHex(digest);
        let low = 0;
        let high = this.#count - 1;
        while (low <= high) {
            const middle = (low + high) >>> 1;
            const order = compareDigest(this.#bytes, FILES_HEADER + middle * FILE_ROW, wanted);
            if (order < 0) {
                low = middle + 1;
            } else if (order > 0) {
                high = middle - 1;
            } else {
                return this.#row(middle, digest);
            }
        }

        return undefined;
    }

    /** Read a row's file: its byte count, with its offset in the packs or its path. */
    #row(index: number, digest: Digest): BuildLayerFile {
        // read the byte count, and a packed file's offset
        const row = FILES_HEADER + index * FILE_ROW;
        const size = this.#view.getUint32(row + DIGEST_BYTES, true);
        const end = this.#view.getUint32(row + DIGEST_BYTES + 4, true);
        if (this.#addressing === "pack") {
            return { digest, size, addressing: "pack", offset: end };
        }

        // read the path between the previous row's end and this row's
        const paths = FILES_HEADER + this.#count * FILE_ROW;
        const start = index === 0 ? 0 : this.#view.getUint32(row - 4, true);
        const path = new TextDecoder().decode(this.#bytes.subarray(paths + start, paths + end));

        return { digest, size, addressing: "path", path };
    }
}

/** Compare the digest at an offset of some bytes with a wanted digest's bytes. */
function compareDigest(bytes: Uint8Array, offset: number, wanted: Uint8Array): number {
    for (let index = 0; index < DIGEST_BYTES; index += 1) {
        const difference = (bytes[offset + index] ?? 0) - (wanted[index] ?? 0);
        if (difference !== 0) {
            return difference;
        }
    }

    return 0;
}

/** The origin a Worker reads its static assets at through their binding, which ignores the host. */
const ASSETS_ORIGIN = "https://assets.invalid";

/** Where a layer's files lie: a directory on disk, or the static assets a Worker version ships. */
type BuildLayerSource =
    | {
          /** A directory on disk. */
          readonly kind: "directory";
          /** The directory. */
          readonly directory: string;
          /** The files reading it, and the crypto verifying what they read. */
          readonly environment: Pick<Environment<"files">, "files" | "crypto">;
      }
    | {
          /** The static assets a Worker version ships, a packed layer. */
          readonly kind: "assets";
          /** The assets' binding. */
          readonly assets: AssetBinding;
          /** The crypto verifying what it answers. */
          readonly crypto: Environment["crypto"];
      };

/** The binding of a Worker's static assets, answering a file by its URL's path. */
export interface AssetBinding {
    /** Fetch a file. */
    readonly fetch: (input: string) => Promise<Response>;
}

/** Read-only builds a store reads in place, as Nix substituters and OCI layers lend theirs. */
export class BuildLayer {
    /** Where the layer's files lie, none for an empty layer. */
    readonly #source: BuildLayerSource | undefined;
    /** The index of the layer's releases and builds. */
    readonly #index: BuildLayerIndex;
    /** The table of the layer's files. */
    readonly #files: BuildLayerTable;
    /** The files each build distributes: rows of build, file and byte count, sorted by build. */
    readonly #references: Uint8Array;
    /** The digest of the index naming the builds the layer ships, absent for an empty layer. */
    readonly digest: Digest | undefined;
    /** The packs read last, by number, the least recently read first. */
    readonly #packs = new Map<number, Promise<Uint8Array<ArrayBuffer>>>();

    /** Create a layer from where its files lie and its index. */
    private constructor(
        source: BuildLayerSource | undefined,
        index: BuildLayerIndex,
        files: BuildLayerTable,
        references: Uint8Array,
        digest: Digest | undefined,
    ) {
        // take where the files lie, the index, the two tables and the index's digest
        this.#source = source;
        this.#index = index;
        this.#files = files;
        this.#references = references;
        this.digest = digest;
    }

    /** Make a layer without builds. */
    static empty(): BuildLayer {
        return new BuildLayer(
            undefined,
            {
                formatVersion: LAYER_FORMAT,
                releases: [],
                images: {},
                builds: {},
                addressing: "path",
                packs: [],
            },
            new BuildLayerTable(new Uint8Array(), "path"),
            new Uint8Array(),
            undefined,
        );
    }

    /** Open the layer a Worker version ships as its static assets, through their binding. */
    static async assets(assets: AssetBinding, crypto: Environment["crypto"]): Promise<BuildLayer> {
        // read the index and the two tables
        const text = await (await BuildLayer.#fetch(assets, LAYER_INDEX)).text();
        const index = BuildLayerIndex.parse(JSON.parse(text));
        const table = await (await BuildLayer.#fetch(assets, LAYER_FILES)).arrayBuffer();
        const references = await (await BuildLayer.#fetch(assets, LAYER_REFERENCES)).arrayBuffer();

        return new BuildLayer(
            { kind: "assets", assets, crypto },
            index,
            new BuildLayerTable(new Uint8Array(table), index.addressing),
            new Uint8Array(references),
            await Digest.of(text, crypto),
        );
    }

    /** Open the layer in a directory by its index, opening no build. */
    static async open(
        directory: string,
        environment: Pick<Environment<"files">, "files" | "crypto">,
    ): Promise<BuildLayer> {
        // read the index and the two tables
        const { files } = environment;
        const text = await files.readTextFile(files.join(directory, LAYER_INDEX));
        const index = BuildLayerIndex.parse(JSON.parse(text));
        const table = await files.readFile(files.join(directory, LAYER_FILES));
        const references = await files.readFile(files.join(directory, LAYER_REFERENCES));

        return new BuildLayer(
            { kind: "directory", directory, environment },
            index,
            new BuildLayerTable(table, index.addressing),
            references,
            await Digest.of(text, environment.crypto),
        );
    }

    /** Write a layer directory's two tables, then its index of its images, builds and packs. */
    static async writeIndex(
        directory: string,
        images: ReadonlyMap<Digest, BuildImage>,
        builds: BuildLayerIndex["builds"],
        references: ReadonlyMap<Digest, readonly Pick<PackageFile, "digest" | "size">[]>,
        addressing: BuildLayerAddressing,
        placed: readonly BuildLayerFile[],
        packs: readonly number[],
        files: FileSystem,
    ): Promise<void> {
        // write each build's distinct referenced files as rows sorted by build, then by file
        const rows = [...references]
            .flatMap(([build, listed]) =>
                [...new Map(listed.map((file) => [file.digest, file])).values()].map((file) => ({
                    build,
                    digest: file.digest,
                    size: file.size,
                })),
            )
            .toSorted((left, right) =>
                `${left.build}${left.digest}` < `${right.build}${right.digest}` ? -1 : 1,
            );
        const table = new Uint8Array(rows.length * REFERENCE_ROW);
        const view = new DataView(table.buffer);
        for (const [index, row] of rows.entries()) {
            // refuse a byte count a row cannot hold
            if (row.size > MAXIMUM_FILE_BYTES) {
                throw PackageError.of(
                    "LAYER_FILE_BROKEN",
                    `the layer cannot list file '${row.digest}': its '${String(row.size)}' bytes exceed a row's u32`,
                );
            }
            table.set(Uint8Array.fromHex(row.build), index * REFERENCE_ROW);
            table.set(Uint8Array.fromHex(row.digest), index * REFERENCE_ROW + DIGEST_BYTES);
            view.setUint32(index * REFERENCE_ROW + 2 * DIGEST_BYTES, row.size, true);
        }
        await files.writeFile(files.join(directory, LAYER_REFERENCES), table);

        // write the file table, then the index last
        await files.writeFile(files.join(directory, LAYER_FILES), BuildLayerTable.encode(placed));
        const index: BuildLayerIndex = {
            formatVersion: LAYER_FORMAT,
            releases: [...images.keys()],
            images: Object.fromEntries(images),
            builds,
            addressing,
            packs: [...packs],
        };
        await files.writeTextFile(files.join(directory, LAYER_INDEX), JSON.stringify(index));
    }

    /** Report whether the layer ships no builds. */
    get isEmpty(): boolean {
        return this.#source === undefined;
    }

    /** List the images of the releases the layer ships, each with what it runs. */
    get releases(): readonly Digest[] {
        return this.#index.releases;
    }

    /** Read a shipped image, absent for an image outside the layer. */
    image(digest: Digest): BuildImage | undefined {
        return this.#index.images[digest];
    }

    /** List the files a layered build references, absent for a build outside the layer. */
    references(build: Digest): { readonly digest: Digest; readonly size: number }[] | undefined {
        // answer nothing outside the layer
        if (!this.hasBuild(build)) {
            return undefined;
        }

        // find the build's first row among the rows sorted by build
        const wanted = Uint8Array.fromHex(build);
        const rows = this.#references;
        const count = rows.byteLength / REFERENCE_ROW;
        let low = 0;
        let high = count;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (compareDigest(rows, middle * REFERENCE_ROW, wanted) < 0) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        // read its rows, each a file's digest and byte count
        const view = new DataView(rows.buffer, rows.byteOffset, rows.byteLength);
        const listed: { readonly digest: Digest; readonly size: number }[] = [];
        for (let index = low; index < count; index += 1) {
            const row = index * REFERENCE_ROW;
            if (compareDigest(rows, row, wanted) !== 0) {
                break;
            }
            const file = rows.subarray(row + DIGEST_BYTES, row + 2 * DIGEST_BYTES);
            listed.push({
                digest: Digest.parse(file.toHex()),
                size: view.getUint32(row + 2 * DIGEST_BYTES, true),
            });
        }

        return listed;
    }

    /** List the shipped images listing a build. */
    referrers(digest: Digest): Digest[] {
        return Object.entries(this.#index.images)
            .filter(([, image]) => BuildImage.builds(image).includes(digest))
            .map(([image]) => Digest.parse(image));
    }

    /** List the real directories the layer's files lie in: its own, its trees' and its builds'. */
    async directories(): Promise<readonly string[]> {
        // read none for a layer off disk
        const source = this.#source;
        if (source?.kind !== "directory") {
            return [];
        }

        // follow the layer, its trees and each build to where they lie, each directory once
        const { files } = source.environment;
        const linked = [
            source.directory,
            files.join(source.directory, LAYER_TREES),
            ...this.builds().map((digest) => files.join(source.directory, digest)),
        ];
        const real = await Promise.all(linked.map((path) => files.realPath(path)));
        const parents = real.slice(2).map((path) => path.slice(0, path.lastIndexOf("/")));

        return [...new Set([...real.slice(0, 2), ...parents])];
    }

    /** List the layered builds' manifest digests. */
    builds(): readonly Digest[] {
        // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- a layer keys its builds by digest
        return Object.keys(this.#index.builds) as Digest[];
    }

    /** Report whether the layer has a build by its manifest's digest. */
    hasBuild(digest: Digest): boolean {
        return Object.hasOwn(this.#index.builds, digest);
    }

    /** Open a layered build by its manifest's digest, absent for a build outside the layer. */
    build(digest: Digest, missing?: BuildReader["stream"]): Promise<BuildReader> | undefined {
        // answer nothing outside the layer
        if (!this.hasBuild(digest)) {
            return undefined;
        }
        return this.#open(digest, missing);
    }

    /** Find where a layered file lies on disk, absent outside the layer or off disk. */
    file(digest: Digest): string | undefined {
        const source = this.#source;
        const located = this.#files.find(digest);

        return source?.kind === "directory" && located?.addressing === "path"
            ? source.environment.files.join(source.directory, located.path)
            : undefined;
    }

    /** Describe a layered build's manifest or a layered image, absent for one outside the layer. */
    head(digest: Digest): PackageFile | undefined {
        const size =
            this.hasBuild(digest) || Object.hasOwn(this.#index.images, digest)
                ? this.size(digest)
                : undefined;

        return size === undefined
            ? undefined
            : { path: MANIFEST_PATH, digest, size, mediaType: "application/json" };
    }

    /** Read a layered blob's byte count: an image's, a manifest's or a file's, absent for none. */
    size(digest: Digest): number | undefined {
        const image = this.#index.images[digest];

        return image === undefined
            ? (this.#index.builds[digest] ?? this.#files.find(digest)?.size)
            : BuildImage.serialize(image).byteLength;
    }

    /** Read a layered blob whole, absent for one outside the layer. */
    read(digest: Digest): Promise<Uint8Array<ArrayBuffer>> | undefined {
        // serve an image's canonical bytes, else read a manifest or a file where it lies
        const image = this.#index.images[digest];
        const located = this.#files.find(digest);
        if (image !== undefined) {
            return Promise.resolve(BuildImage.serialize(image));
        } else if (this.hasBuild(digest)) {
            return this.#read(this.#manifestPath(digest));
        } else if (located !== undefined) {
            return this.#located(located);
        } else {
            return undefined;
        }
    }

    /** Stream a layered blob, absent for one outside the layer. */
    stream(digest: Digest): ReadableStream<Uint8Array> | undefined {
        // read it whole as the stream starts
        const reading = this.read(digest);
        if (reading === undefined) {
            return undefined;
        }

        return new ReadableStream<Uint8Array>({
            async start(controller) {
                controller.enqueue(await reading);
                controller.close();
            },
        });
    }

    /** Open a layered build where it lies, streaming the files it leaves out from elsewhere. */
    async #open(digest: Digest, missing: BuildReader["stream"] | undefined): Promise<BuildReader> {
        // open a build on disk from its directory
        const source = present(this.#source, "the files of an empty layer");
        if (source.kind === "directory") {
            const { files, crypto } = source.environment;
            const directory = files.join(source.directory, digest);

            return await BuildReader.openDirectory(directory, files, crypto, missing);
        }

        // open one in assets by its manifest
        const manifest = PackageManifest.parse(
            await BuildLayer.#fetchJson(source.assets, this.#manifestPath(digest)),
        );
        const reader: BuildReader = new BuildReader(
            manifest,
            async (path, signal) => {
                // stream a layered file by its digest, else through the caller's stream
                const listed = await reader.file(path);
                const located = this.#files.find(listed.digest);
                if (located === undefined) {
                    return present(missing, `the layered file ${path}`)(path, signal);
                }
                const bytes = await this.#located(located);

                return new Blob([bytes]).stream();
            },
            source.crypto,
        );

        return reader;
    }

    /** Read a file where the table finds it: a file of its own, or a slice of its pack. */
    async #located(located: BuildLayerFile): Promise<Uint8Array<ArrayBuffer>> {
        // read a file of its own whole, and an empty packed file from no pack
        if (located.addressing === "path") {
            return await this.#read(located.path);
        } else if (located.size === 0) {
            return new Uint8Array();
        }

        // find the pack the file lies in, by the packs' byte counts in order
        const { packs } = this.#index;
        let number = 0;
        let start = 0;
        while (number < packs.length && located.offset >= start + (packs[number] ?? 0)) {
            start += packs[number] ?? 0;
            number += 1;
        }
        if (located.offset + located.size > start + (packs[number] ?? 0)) {
            throw PackageError.of(
                "LAYER_FILE_BROKEN",
                `the layer has no pack for '${String(located.size)}' bytes at '${String(located.offset)}'`,
            );
        }

        // copy the file's bytes out of its pack
        const pack = await this.#pack(number);
        const from = located.offset - start;

        return pack.slice(from, from + located.size);
    }

    /** Read a pack whole once, keeping the packs read last within the kept bytes. */
    #pack(number: number): Promise<Uint8Array<ArrayBuffer>> {
        // read the pack unless it is kept, mark it read last, and forget a read that fails
        const reading = this.#packs.get(number) ?? this.#read(`${LAYER_PACKS}/${String(number)}`);
        this.#packs.delete(number);
        this.#packs.set(number, reading);
        reading.catch(() => this.#packs.delete(number));

        // drop the packs read longest ago past the kept bytes, keeping the one just read
        const sizes = this.#index.packs;
        let kept = [...this.#packs.keys()].reduce((sum, each) => sum + (sizes[each] ?? 0), 0);
        for (const earlier of this.#packs.keys()) {
            if (kept <= KEPT_PACK_BYTES || earlier === number) {
                break;
            }
            this.#packs.delete(earlier);
            kept -= sizes[earlier] ?? 0;
        }

        return reading;
    }

    /** Lay groups of files out into packs: a group's files together, a new pack at the limit. */
    static pack(
        groups: readonly (readonly Pick<BuildLayerFile, "digest" | "size">[])[],
        limit: number,
    ): BuildLayerFile[][] {
        // place each file after the one before it, closing a pack at a group's end or the limit
        const packs: BuildLayerFile[][] = [];
        let offset = 0;
        for (const group of groups) {
            let pack: BuildLayerFile[] = [];
            let packed = 0;
            for (const file of group) {
                // start the next pack once this file passes the limit
                if (pack.length > 0 && packed + file.size > limit) {
                    packs.push(pack);
                    pack = [];
                    packed = 0;
                }
                pack.push({ digest: file.digest, size: file.size, addressing: "pack", offset });
                packed += file.size;
                offset += file.size;
            }
            if (pack.length > 0) {
                packs.push(pack);
            }
        }

        return packs;
    }

    /** Find a layered build's manifest below the layer: a blob in assets, else in its build. */
    #manifestPath(digest: Digest): string {
        return this.#source?.kind === "assets"
            ? `${LAYER_BLOBS}/${digest}`
            : `${digest}/${MANIFEST_PATH}`;
    }

    /** Read a file below the layer whole. */
    async #read(path: string): Promise<Uint8Array<ArrayBuffer>> {
        const source = present(this.#source, "the files of an empty layer");

        return source.kind === "directory"
            ? await source.environment.files.readFile(
                  source.environment.files.join(source.directory, path),
              )
            : new Uint8Array(await (await BuildLayer.#fetch(source.assets, path)).arrayBuffer());
    }

    /** Fetch a file of a layer in assets, refusing one they lack. */
    static async #fetch(assets: AssetBinding, path: string): Promise<Response> {
        const response = await assets.fetch(`${ASSETS_ORIGIN}/${path}`);
        if (!response.ok) {
            throw PackageError.of(
                "LAYER_FILE_BROKEN",
                `the layer has no file '${path}': its assets answered ${response.status}`,
            );
        }

        return response;
    }

    /** Fetch and parse a JSON file of a layer in assets. */
    static async #fetchJson(assets: AssetBinding, path: string): Promise<unknown> {
        return JSON.parse(await (await BuildLayer.#fetch(assets, path)).text());
    }
}
