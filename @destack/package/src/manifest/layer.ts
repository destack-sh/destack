import type { Environment, FileSystem } from "@destack/environment";
import type { PackageFile } from "../file/index.ts";
import { PackageError } from "../error/index.ts";
import { Lock } from "./lock.ts";
import { PackageManifest } from "./manifest.ts";
import { BuildReader, MANIFEST_PATH } from "./reader.ts";
import { Digest, present, schema } from "@destack/schema";

/** The file of a layer directory indexing its releases, builds and files. */
export const LAYER_INDEX = "index.json";

/** The layer index format a store writes. */
export const LAYER_FORMAT = 4;

/** The directory of a layer with its releases' dependency trees, laid out once on writing. */
export const LAYER_TREES = "trees";

/** The directory of an addressed layer's manifests and files, each named by its digest. */
export const LAYER_BLOBS = "blobs";

/** The index of a layer directory, written once with it, so opening it reads one file. */
export const LayerIndex = schema.object({
    /** The layer index format version. */
    formatVersion: schema.literal(LAYER_FORMAT),
    /** The locks of the releases the layer ships, each with what it runs. */
    releases: schema.array(Digest),
    /** Each shipped lock, by its digest. */
    locks: schema.record(Digest, Lock),
    /** The byte count of each layered build's manifest, by the manifest's digest. */
    builds: schema.record(Digest, schema.int()),
    /** Where each layered file lies below the layer and its byte count, by digest. */
    files: schema.record(
        Digest,
        schema.object({
            /** The file's path below the layer. */
            path: schema.string().min(1),
            /** The file's byte count. */
            size: schema.int(),
        }),
    ),
});
/** The index of a layer directory. */
export type LayerIndex = schema.Infer<typeof LayerIndex>;

/** The origin a Worker reads its static assets at through their binding, which ignores the host. */
const ASSETS_ORIGIN = "https://assets.invalid";

/** Where a layer's files lie: a directory on disk, or the static assets a Worker version ships. */
type LayerSource =
    | {
          /** A directory on disk, which runtimes run builds in place from. */
          readonly kind: "directory";
          /** The directory. */
          readonly directory: string;
          /** The files reading it, and the crypto verifying what they read. */
          readonly environment: Pick<Environment<"files">, "files" | "crypto">;
      }
    | {
          /** The static assets a Worker version ships, an addressed layer. */
          readonly kind: "assets";
          /** The assets' binding. */
          readonly assets: LayerAssets;
          /** The crypto verifying what it answers. */
          readonly crypto: Environment["crypto"];
      };

/** The binding of a Worker's static assets, answering a file by its URL's path. */
export interface LayerAssets {
    /** Fetch a file. */
    readonly fetch: (input: string) => Promise<Response>;
}

/** Read-only builds a store reads in place, as Nix substituters and OCI layers lend theirs. */
export class BuildLayer {
    /** Where the layer's files lie, none for an empty layer. */
    readonly #source: LayerSource | undefined;
    /** The index of the layer's releases, builds and files. */
    readonly #index: LayerIndex;
    /** The layered builds' readers opened so far, each once, by manifest digest. */
    readonly #readers = new Map<Digest, Promise<BuildReader>>();

    /** Create a layer from where its files lie and its index. */
    private constructor(source: LayerSource | undefined, index: LayerIndex) {
        this.#source = source;
        this.#index = index;
    }

    /** Make a layer without builds. */
    static empty(): BuildLayer {
        return new BuildLayer(undefined, {
            formatVersion: LAYER_FORMAT,
            releases: [],
            locks: {},
            builds: {},
            files: {},
        });
    }

    /** Open the layer a Worker version ships as its static assets, through their binding. */
    static async assets(assets: LayerAssets, crypto: Environment["crypto"]): Promise<BuildLayer> {
        const index = LayerIndex.parse(await BuildLayer.#fetchJson(assets, LAYER_INDEX));

        return new BuildLayer({ kind: "assets", assets, crypto }, index);
    }

    /** Open the layer in a directory by its index, opening no build. */
    static async open(
        directory: string,
        environment: Pick<Environment<"files">, "files" | "crypto">,
    ): Promise<BuildLayer> {
        const { files } = environment;
        const index = LayerIndex.parse(
            JSON.parse(await files.readTextFile(files.join(directory, LAYER_INDEX))),
        );

        return new BuildLayer({ kind: "directory", directory, environment }, index);
    }

    /** Write a layer directory's index of its locks, each build's manifest size and its files. */
    static async writeIndex(
        directory: string,
        locks: ReadonlyMap<Digest, Lock>,
        builds: LayerIndex["builds"],
        placed: LayerIndex["files"],
        files: FileSystem,
    ): Promise<void> {
        const index: LayerIndex = {
            formatVersion: LAYER_FORMAT,
            releases: [...locks.keys()],
            locks: Object.fromEntries(locks),
            builds,
            files: placed,
        };
        await files.writeTextFile(files.join(directory, LAYER_INDEX), JSON.stringify(index));
    }

    /** Report whether the layer ships no builds. */
    get isEmpty(): boolean {
        return this.#source === undefined;
    }

    /** List the locks of the releases the layer ships, each with what it runs. */
    get releases(): readonly Digest[] {
        return this.#index.releases;
    }

    /** Read a shipped lock, absent for a lock outside the layer. */
    lock(digest: Digest): Lock | undefined {
        return this.#index.locks[digest];
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

    /** Open a layered build once by its manifest's digest, absent for a build outside the layer. */
    build(digest: Digest, missing?: BuildReader["stream"]): Promise<BuildReader> | undefined {
        // answer nothing outside the layer, and each build's one reader
        if (!this.hasBuild(digest)) {
            return undefined;
        }
        let opened = this.#readers.get(digest);
        if (opened === undefined) {
            opened = this.#open(digest, missing);
            this.#readers.set(digest, opened);
        }

        return opened;
    }

    /** Find a layered build's directory or a lock's run directory, absent off disk. */
    directory(digest: Digest): string | undefined {
        const source = this.#source;
        const isLayered = this.hasBuild(digest) || Object.hasOwn(this.#index.locks, digest);

        return source?.kind === "directory" && isLayered
            ? source.environment.files.join(source.directory, digest)
            : undefined;
    }

    /** Find where a layered file lies on disk, absent outside the layer or off disk. */
    file(digest: Digest): string | undefined {
        const source = this.#source;
        const located = this.#index.files[digest];

        return source?.kind === "directory" && located !== undefined
            ? source.environment.files.join(source.directory, located.path)
            : undefined;
    }

    /** Describe a layered build's manifest or a layered lock, absent for one outside the layer. */
    head(digest: Digest): PackageFile | undefined {
        const size =
            this.hasBuild(digest) || Object.hasOwn(this.#index.locks, digest)
                ? this.size(digest)
                : undefined;

        return size === undefined
            ? undefined
            : { path: MANIFEST_PATH, digest, size, mediaType: "application/json" };
    }

    /** Read a layered blob's byte count: a lock's, a manifest's or a file's, absent for none. */
    size(digest: Digest): number | undefined {
        const lock = this.#index.locks[digest];

        return lock === undefined
            ? (this.#index.builds[digest] ?? this.#index.files[digest]?.size)
            : Lock.serialize(lock).byteLength;
    }

    /** Read a layered blob whole, absent for one outside the layer. */
    read(digest: Digest): Promise<Uint8Array<ArrayBuffer>> | undefined {
        // serve a lock's canonical bytes, else read a manifest's or a file's path below the layer
        const lock = this.#index.locks[digest];
        const path = this.hasBuild(digest)
            ? this.#manifestPath(digest)
            : this.#index.files[digest]?.path;
        if (lock !== undefined) {
            return Promise.resolve(Lock.serialize(lock));
        } else if (path !== undefined) {
            return this.#read(path);
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
                const located = this.#index.files[listed.digest];

                return located === undefined
                    ? present(missing, `the layered file ${path}`)(path, signal)
                    : present((await BuildLayer.#fetch(source.assets, located.path)).body, path);
            },
            source.crypto,
        );

        return reader;
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
    static async #fetch(assets: LayerAssets, path: string): Promise<Response> {
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
    static async #fetchJson(assets: LayerAssets, path: string): Promise<unknown> {
        return JSON.parse(await (await BuildLayer.#fetch(assets, path)).text());
    }
}
