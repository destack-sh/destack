import type { Environment, FileSystem } from "@destack/environment";
import { BuildReader, MANIFEST_PATH, PackageManifest } from "@destack/package/manifest";
import type { PackageFile } from "@destack/package/file";
import { HOST_RUNTIME, type Runtime } from "@destack/package/runtime";
import { Digest, present, schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";

/** The file of a layer directory indexing its releases, builds and files. */
export const LAYER_INDEX = "index.json";

/** The directory of a layer with its releases' dependency trees, laid out once on writing. */
export const LAYER_TREES = "trees";

/** The directory of an addressed layer's manifests and files, each named by its digest. */
export const LAYER_BLOBS = "blobs";

/** The index of a layer directory, written once with it, so opening it reads one file. */
export const LayerIndex = schema.object({
    /** The releases the layer ships, each with what it runs. */
    releases: schema.array(Digest),
    /** The byte count of each layered build's manifest, by the manifest's digest. */
    builds: schema.record(Digest, schema.int()),
    /** Where each layered file lies below the layer, as its build's digest and path, by digest. */
    files: schema.record(Digest, schema.string().min(1)),
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
    /** The trees laid out with the layer on disk, by their roots' names. */
    readonly #trees: ReadonlyMap<string, string>;
    /** The layered builds' readers opened so far, each once, by manifest digest. */
    readonly #readers = new Map<Digest, Promise<BuildReader>>();

    /** Create a layer from where its files lie, its index and its trees. */
    private constructor(
        source: LayerSource | undefined,
        index: LayerIndex,
        trees: ReadonlyMap<string, string>,
    ) {
        this.#source = source;
        this.#index = index;
        this.#trees = trees;
    }

    /** Make a layer without builds. */
    static empty(): BuildLayer {
        return new BuildLayer(undefined, { releases: [], builds: {}, files: {} }, new Map());
    }

    /** Open the layer a Worker version ships as its static assets, through their binding. */
    static async assets(assets: LayerAssets, crypto: Environment["crypto"]): Promise<BuildLayer> {
        const index = LayerIndex.parse(await BuildLayer.#fetchJson(assets, LAYER_INDEX));

        return new BuildLayer({ kind: "assets", assets, crypto }, index, new Map());
    }

    /** Open the layer in a directory by its index and its trees, opening no build. */
    static async open(
        directory: string,
        environment: Pick<Environment<"files">, "files" | "crypto">,
    ): Promise<BuildLayer> {
        // read the index, then find the trees laid out with it by their roots' names
        const { files } = environment;
        const index = LayerIndex.parse(
            JSON.parse(await files.readTextFile(files.join(directory, LAYER_INDEX))),
        );
        const trees = files.join(directory, LAYER_TREES);
        const roots = (await files.readDirectory(trees)).filter(
            (entry) => entry.kind === "directory",
        );

        return new BuildLayer(
            { kind: "directory", directory, environment },
            index,
            new Map(roots.map((entry) => [entry.name, files.join(trees, entry.name)])),
        );
    }

    /** Write the index of builds written below a layer directory, listing each build's files. */
    static async writeIndex(
        directory: string,
        releases: readonly Digest[],
        builds: ReadonlyMap<Digest, BuildReader>,
        runtimes: readonly Runtime[],
        files: FileSystem,
    ): Promise<void> {
        // locate each build's manifest and every file it distributes below the layer
        const index: LayerIndex = { releases: [...releases], builds: {}, files: {} };
        for (const [digest, reader] of builds) {
            const manifest = await files.stat(files.join(directory, digest, MANIFEST_PATH));
            index.builds[digest] = manifest.size;
            for (const file of await BuildLayer.shipped(reader, runtimes)) {
                index.files[file.digest] ??= `${digest}/${file.path}`;
            }
        }
        await files.writeTextFile(files.join(directory, LAYER_INDEX), JSON.stringify(index));
    }

    /** List the releases the layer ships, each with what it runs. */
    get releases(): readonly Digest[] {
        return this.#index.releases;
    }

    /** List what a layer ships of a build: lists, its runtimes' outputs, sources for the host. */
    static async shipped(
        reader: BuildReader,
        runtimes: readonly Runtime[],
    ): Promise<PackageFile[]> {
        // take the outputs on the shipped runtimes, leaving their maps, which the store reads on demand
        const outputs = Object.entries(reader.manifest.outputs);
        const shipped = outputs.filter(([, output]) => runtimes.includes(output.runtime));
        const directories = outputs.map(([, output]) => `${output.directory}/`);
        const kept = shipped.map(([, output]) => `${output.directory}/`);
        const maps = await Promise.all(shipped.map(([name]) => reader.maps(name)));
        const left = new Set(maps.flat().map((file) => file.path));

        // take the lists, and the files outside every output, such as sources, for the host alone
        const isHosting = runtimes.includes(HOST_RUNTIME);
        const lists = new Set(reader.references().map((file) => file.path));

        return (await reader.distributed()).filter((file) => {
            const isOutput = directories.some((directory) => file.path.startsWith(directory));
            const isKept = kept.some((directory) => file.path.startsWith(directory));

            return (
                lists.has(file.path) || (isKept && !left.has(file.path)) || (!isOutput && isHosting)
            );
        });
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

    /** Find the dependency tree the layer ships for a tree root's name. */
    tree(name: string): string | undefined {
        return this.#trees.get(name);
    }

    /** List the layered builds' manifest digests. */
    builds(): readonly Digest[] {
        return Object.keys(this.#index.builds).map((digest) => Digest.parse(digest));
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

    /** Find a layered build's directory on disk, absent outside the layer or off disk. */
    directory(digest: Digest): string | undefined {
        const source = this.#source;

        return source?.kind === "directory" && this.hasBuild(digest)
            ? source.environment.files.join(source.directory, digest)
            : undefined;
    }

    /** Find where a layered file lies on disk, absent outside the layer or off disk. */
    file(digest: Digest): string | undefined {
        const source = this.#source;
        const located = this.#index.files[digest];

        return source?.kind === "directory" && located !== undefined
            ? source.environment.files.join(source.directory, located)
            : undefined;
    }

    /** Describe a layered build's manifest file, absent for a build outside the layer. */
    head(digest: Digest): PackageFile | undefined {
        const size = this.#index.builds[digest];

        return size === undefined
            ? undefined
            : { path: MANIFEST_PATH, digest, size, mediaType: "application/json" };
    }

    /** Report whether the layer has a blob: a build's manifest or one of its files. */
    has(digest: Digest): boolean {
        return this.hasBuild(digest) || Object.hasOwn(this.#index.files, digest);
    }

    /** Stream a layered blob, absent for one outside the layer. */
    stream(digest: Digest): ReadableStream<Uint8Array> | undefined {
        // find a manifest's path or a file's below the layer, none outside it
        const path = this.hasBuild(digest) ? this.#manifestPath(digest) : this.#index.files[digest];
        if (path === undefined) {
            return undefined;
        }
        const read = () => this.#read(path);

        // read it whole as the stream starts
        return new ReadableStream<Uint8Array>({
            async start(controller) {
                controller.enqueue(await read());
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

        // open one in assets by its manifest, streaming each file it lists by its digest, else from where the caller keeps it
        const manifest = PackageManifest.parse(
            await BuildLayer.#fetchJson(source.assets, this.#manifestPath(digest)),
        );
        const reader: BuildReader = new BuildReader(
            manifest,
            async (path, signal) => {
                // find the file's digest among the manifest's lists, else its listed files
                const listed =
                    reader.references().find((file) => file.path === path) ??
                    (await reader.files()).find((file) => file.path === path);
                const located = listed === undefined ? undefined : this.#index.files[listed.digest];

                return located === undefined
                    ? present(missing, `the layered file ${path}`)(path, signal)
                    : present((await BuildLayer.#fetch(source.assets, located)).body, path);
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
            throw BuildError.of(
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
