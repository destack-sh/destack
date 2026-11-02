import type { Environment, FileSystem } from "@destack/environment";
import { BuildReader, MANIFEST_PATH } from "@destack/package/manifest";
import type { PackageFile } from "@destack/package/file";
import { Digest, present, schema } from "@destack/schema";

/** The file of a layer directory indexing its releases, builds and files. */
export const LAYER_INDEX = "index.json";

/** The directory of a layer with its releases' dependency trees, laid out once on writing. */
export const LAYER_TREES = "trees";

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

/** Read-only builds a store reads in place, as Nix substituters and OCI layers lend theirs. */
export class BuildLayer {
    /** The directory the layer lies in, none for an empty layer. */
    readonly #directory: string;
    /** The index of the layer's releases, builds and files. */
    readonly #index: LayerIndex;
    /** The trees laid out with the layer, by their roots' names. */
    readonly #trees: ReadonlyMap<string, string>;
    /** The environment reading the layer's files, absent for an empty layer, which has none. */
    readonly #environment: Pick<Environment<"files">, "files" | "crypto"> | undefined;
    /** The layered builds' readers opened so far, each once, by manifest digest. */
    readonly #readers = new Map<Digest, Promise<BuildReader>>();

    /** Create a layer from its directory, index and trees. */
    private constructor(
        directory: string,
        index: LayerIndex,
        trees: ReadonlyMap<string, string>,
        environment: Pick<Environment<"files">, "files" | "crypto"> | undefined,
    ) {
        // set the directory, the index, the trees and the files reading them
        this.#directory = directory;
        this.#index = index;
        this.#trees = trees;
        this.#environment = environment;
    }

    /** The environment reading the layer's files, which only a layer with files reaches. */
    get #reading(): Pick<Environment<"files">, "files" | "crypto"> {
        return present(this.#environment, "the files of an empty layer");
    }

    /** Make a layer without builds. */
    static empty(): BuildLayer {
        return new BuildLayer("", { releases: [], builds: {}, files: {} }, new Map(), undefined);
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
            directory,
            index,
            new Map(roots.map((entry) => [entry.name, files.join(trees, entry.name)])),
            environment,
        );
    }

    /** Write the index of builds written below a layer directory, listing each build's files. */
    static async writeIndex(
        directory: string,
        releases: readonly Digest[],
        builds: ReadonlyMap<Digest, BuildReader>,
        files: FileSystem,
    ): Promise<void> {
        // locate each build's manifest and every file it distributes below the layer
        const index: LayerIndex = { releases: [...releases], builds: {}, files: {} };
        for (const [digest, reader] of builds) {
            const manifest = await files.stat(files.join(directory, digest, MANIFEST_PATH));
            index.builds[digest] = manifest.size;
            for (const file of await reader.distributed()) {
                index.files[file.digest] ??= `${digest}/${file.path}`;
            }
        }
        await files.writeTextFile(files.join(directory, LAYER_INDEX), JSON.stringify(index));
    }

    /** List the releases the layer ships, each with what it runs. */
    get releases(): readonly Digest[] {
        return this.#index.releases;
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
    build(digest: Digest): Promise<BuildReader> | undefined {
        // answer nothing outside the layer, and each build's one reader
        if (!this.hasBuild(digest)) {
            return undefined;
        }
        let opened = this.#readers.get(digest);
        if (opened === undefined) {
            const { files, crypto } = this.#reading;
            opened = BuildReader.openDirectory(files.join(this.#directory, digest), files, crypto);
            this.#readers.set(digest, opened);
        }

        return opened;
    }

    /** Find where a layered file lies on disk by its digest, absent for one outside the layer. */
    file(digest: Digest): string | undefined {
        const located = this.#index.files[digest];

        return located === undefined
            ? undefined
            : this.#reading.files.join(this.#directory, located);
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

    /** Stream a layered blob from disk, absent for one outside the layer. */
    stream(digest: Digest): ReadableStream<Uint8Array> | undefined {
        // find a manifest's file or a file's location, none outside the layer
        const location = this.hasBuild(digest)
            ? this.#reading.files.join(this.#directory, digest, MANIFEST_PATH)
            : this.file(digest);
        if (location === undefined) {
            return undefined;
        }
        const { files } = this.#reading;

        // read it whole as the stream starts
        return new ReadableStream<Uint8Array>({
            async start(controller) {
                controller.enqueue(await files.readFile(location));
                controller.close();
            },
        });
    }
}
