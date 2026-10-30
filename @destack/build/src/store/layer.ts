import { join } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import { BuildReader, MANIFEST_PATH, PackageManifest } from "@destack/package/manifest";
import { PackageFile } from "@destack/package/file";
import { Digest, type DigestCrypto, schema } from "@destack/schema";
import { encodeManifest } from "./store.ts";

/** The file of a layer directory listing its releases and referrers. */
const INDEX_PATH = "index.json";

/** A file attached to a subject build at a path, as OCI's referrers attach artifacts. */
export const BuildReferrer = schema.object({
    /** The build the file is attached to. */
    subject: Digest,
    /** The attached file: its path below the subject, its digest, size and media type. */
    file: PackageFile,
});
/** A file attached to a subject build at a path. */
export type BuildReferrer = schema.Infer<typeof BuildReferrer>;

/** The index of a layer directory: its releases and the files attached to its builds. */
const LayerIndex = schema.object({
    /** The releases the layer ships, each with what it runs. */
    releases: schema.array(Digest),
    /** The files attached to the layered builds. */
    referrers: schema.array(BuildReferrer),
});

/** A layered build's file: the reader holding it and its path there. */
interface LayerFile {
    /** The build reader holding the file. */
    readonly reader: BuildReader;
    /** The file's path in that build. */
    readonly path: string;
}

/** Read-only builds a store reads in place, as Nix substituters and OCI layers lend theirs. */
export class BuildLayer {
    /** The layered builds by their manifest's digest. */
    readonly #builds: ReadonlyMap<Digest, BuildReader>;
    /** The encoded manifest of each layered build, by its digest. */
    readonly #manifests: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>;
    /** The layered builds' files by their digest. */
    readonly #files: ReadonlyMap<Digest, LayerFile>;
    /** The releases the layer ships, each with what it runs. */
    readonly releases: readonly Digest[];
    /** The files attached to the layered builds. */
    readonly referrers: readonly BuildReferrer[];
    /** The attached files by their subject and path. */
    readonly #attached: ReadonlyMap<string, PackageFile>;

    /** Keep the indexes of a layer's builds. */
    private constructor(
        builds: ReadonlyMap<Digest, BuildReader>,
        manifests: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
        files: ReadonlyMap<Digest, LayerFile>,
        releases: readonly Digest[],
        referrers: readonly BuildReferrer[],
    ) {
        // keep the indexes, and index the referrers by subject and path
        this.#builds = builds;
        this.#manifests = manifests;
        this.#files = files;
        this.releases = releases;
        this.referrers = referrers;
        this.#attached = new Map(
            referrers.map((each) => [`${each.subject} ${each.file.path}`, each.file] as const),
        );
    }

    /** Make a layer without builds. */
    static empty(): BuildLayer {
        return new BuildLayer(new Map(), new Map(), new Map(), [], []);
    }

    /** Index some builds by their manifests' and files' digests, with releases and referrers. */
    static async of(
        readers: readonly BuildReader[],
        releases: readonly Digest[],
        referrers: readonly BuildReferrer[],
        crypto: DigestCrypto,
    ): Promise<BuildLayer> {
        // digest each build's encoded manifest, as a store identifies it
        const builds = new Map<Digest, BuildReader>();
        const manifests = new Map<Digest, Uint8Array<ArrayBuffer>>();
        const files = new Map<Digest, LayerFile>();
        for (const reader of readers) {
            const bytes = encodeManifest(PackageManifest.parse(reader.manifest));
            const digest = await Digest.of(bytes, crypto);
            builds.set(digest, reader);
            manifests.set(digest, bytes);

            // index each file the build distributes by its digest
            for (const file of await reader.distributed()) {
                files.set(file.digest, { reader, path: file.path });
            }
        }

        return new BuildLayer(builds, manifests, files, releases, referrers);
    }

    /** Open the layer a directory keeps: a directory per build and its index. */
    static async open(
        directory: string,
        environment: Pick<Environment<"files">, "files" | "crypto">,
    ): Promise<BuildLayer> {
        // read each build directory and the index
        const { files, crypto } = environment;
        const entries = await files.readDirectory(directory);
        const readers = await Promise.all(
            entries
                .filter((entry) => entry.kind === "directory")
                .map((entry) =>
                    BuildReader.openDirectory(join(directory, entry.name), files, crypto),
                ),
        );
        const indexed = await files.readTextFile(join(directory, INDEX_PATH));
        const { releases, referrers } = LayerIndex.parse(JSON.parse(indexed));

        return await BuildLayer.of(readers, releases, referrers, crypto);
    }

    /** Write the layer to a directory, keeping builds written before and removing others. */
    async write(directory: string, files: FileSystem): Promise<void> {
        // write each build missing its manifest, which a build writes last
        await files.mkdir(directory, { recursive: true });
        const written = await files.readDirectory(directory);
        for (const [digest, reader] of this.#builds) {
            const build = join(directory, digest);
            if (!(await files.exists(join(build, MANIFEST_PATH)))) {
                await files.remove(build, { recursive: true, force: true });
                await reader.writeDirectory(build, files);
            }
        }

        // remove the builds the layer no longer has, then write the index
        const kept = new Set<string>(this.#builds.keys());
        const stale = written.filter(
            (entry) => entry.kind === "directory" && !kept.has(entry.name),
        );
        for (const entry of stale) {
            await files.remove(join(directory, entry.name), { recursive: true });
        }
        const index = { releases: this.releases, referrers: this.referrers };
        await files.writeTextFile(join(directory, INDEX_PATH), JSON.stringify(index));
    }

    /** Find a layered build by its manifest's digest. */
    build(digest: Digest): BuildReader | undefined {
        return this.#builds.get(digest);
    }

    /** Describe a layered build's manifest file, absent for a build outside the layer. */
    head(digest: Digest): PackageFile | undefined {
        const bytes = this.#manifests.get(digest);

        return bytes === undefined
            ? undefined
            : {
                  path: "manifest.json",
                  digest,
                  size: bytes.byteLength,
                  mediaType: "application/json",
              };
    }

    /** Read the file attached to a layered build at a path, absent for one without it. */
    referrer(subject: Digest, path: string): PackageFile | undefined {
        return this.#attached.get(`${subject} ${path}`);
    }

    /** Report whether the layer holds a blob: a build's manifest or one of its files. */
    has(digest: Digest): boolean {
        return this.#manifests.has(digest) || this.#files.has(digest);
    }

    /** Stream a layered blob, absent for one outside the layer. */
    stream(digest: Digest): ReadableStream<Uint8Array> | undefined {
        // stream a manifest's encoded bytes
        const manifest = this.#manifests.get(digest);
        if (manifest !== undefined) {
            return new Blob([manifest]).stream();
        }

        // stream a file from the build holding it
        const file = this.#files.get(digest);
        if (file === undefined) {
            return undefined;
        }
        const { reader, path } = file;

        return new ReadableStream<Uint8Array>({
            async start(controller) {
                controller.enqueue(await reader.load(path));
                controller.close();
            },
        });
    }
}
