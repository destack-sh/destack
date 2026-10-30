import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import type { PackageFile } from "@destack/package/file";
import { Digest, type DigestCrypto } from "@destack/schema";
import { encodeManifest } from "./store.ts";

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

    /** Keep the indexes of a layer's builds. */
    private constructor(
        builds: ReadonlyMap<Digest, BuildReader>,
        manifests: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
        files: ReadonlyMap<Digest, LayerFile>,
    ) {
        this.#builds = builds;
        this.#manifests = manifests;
        this.#files = files;
    }

    /** Index some builds by their manifests' digests and their files' digests. */
    static async of(readers: readonly BuildReader[], crypto: DigestCrypto): Promise<BuildLayer> {
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

        return new BuildLayer(builds, manifests, files);
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
