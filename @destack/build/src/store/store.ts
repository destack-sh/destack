import { type Environment, type FileSystem, Pipe, waitAborted } from "@destack/environment";
import { Digest, type DigestCrypto, found, present, schema } from "@destack/schema";
import type { BlobStorage } from "@destack/resource";
import {
    and,
    defineDatabase,
    eq,
    inArray,
    isNull,
    or,
    type SQLWrapper,
    BLOB_TABLES,
} from "@destack/db";
import type { BlobStore, DatabaseConnection } from "@destack/db/database";
import { type DependencyRelease, graph, type PackageId } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import {
    BuildLayer,
    BuildListCache,
    BuildReader,
    BuildImage,
    MANIFEST_PATH,
    PackageManifest,
} from "@destack/package/manifest";
import type { Runtime } from "@destack/package/runtime";
import { BuildError } from "../error/index.ts";
import { TarballEntry } from "@destack/package/archive";
import { comparePath } from "../build/serialization.ts";
import { BuildCache, CACHE_ROOT_PREFIX } from "../cache/cache.ts";
import { type PackageAccess, serveStore } from "./http.ts";
import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";
import {
    BUILD_STORE_TABLES,
    buildCache,
    buildManifest,
    buildOutput,
    buildReference,
    buildAttachment,
    buildRoot,
} from "./table.ts";

/** The rows one statement binds: 500, far under SQLite's 32,766 parameters at two columns. */
export const BATCH_ROWS = 500;

/** The path prefix a build's module graph files are attached at, each by its module's path. */
const GRAPH_PREFIX = "graph/";

/** The most bytes a write gathers into a pack or an upload sends: 64 MiB, bounding memory. */
export const MAX_WRITE_BYTES = 64 * 1024 * 1024;

/** Where a blob lies, with its byte count: in the store's layer, or in one of its packs. */
export type BlobLocation =
    | { readonly kind: "layered"; readonly size: number }
    | ({ readonly kind: "packed" } & Awaited<ReturnType<BlobStore["placements"]>>[number]);

/** A blob a transfer sends: a file, a build's manifest, or an image. */
export interface BuildObject {
    /** What the blob is: a build's file, manifest or image, or a file attached to a build. */
    readonly kind: "file" | "manifest" | "image" | "attachment";
    /** The blob's digest. */
    readonly digest: Digest;
    /** The blob's size in bytes. */
    readonly size: number;
}

/** List the manifests of some builds, then the images among their roots. */
async function describeRecords(
    store: Pick<BuildStore, "head">,
    roots: readonly Pick<BuildObject, "kind" | "digest">[],
    builds: ReadonlySet<Digest>,
): Promise<BuildObject[]> {
    const manifests = await Promise.all([...builds].map((build) => store.head(build)));
    const images = await Promise.all(
        roots.filter((root) => root.kind === "image").map((root) => store.head(root.digest)),
    );

    return [
        ...manifests.map((file): BuildObject => ({
            kind: "manifest",
            digest: file.digest,
            size: file.size,
        })),
        ...images.map((file): BuildObject => ({
            kind: "image",
            digest: file.digest,
            size: file.size,
        })),
    ];
}

/** The objects transfers send: listing them and their tarball paths. */
export const BuildObject = {
    /** List every object of images and builds from their indexes: files, manifests, then images. */
    async of(
        store: Pick<BuildStore, "head" | "image" | "references">,
        roots: readonly Pick<BuildObject, "kind" | "digest">[],
    ): Promise<BuildObject[]> {
        // take every root's builds once
        const builds = new Set<Digest>();
        for (const root of roots) {
            const image = root.kind === "image" ? await store.image(root.digest) : undefined;
            for (const build of image === undefined ? [root.digest] : BuildImage.builds(image)) {
                builds.add(build);
            }
        }

        // take every file each build references, from its index
        const files = new Map<Digest, BuildObject>();
        for (const build of builds) {
            for (const file of await store.references(build)) {
                files.set(file.digest, file);
            }
        }

        return [...files.values(), ...(await describeRecords(store, roots, builds))];
    },

    /** List the objects some runtimes run of images and builds: files, manifests, then images. */
    async runtime(
        store: Pick<BuildStore, "get" | "contents" | "head" | "image">,
        roots: readonly Pick<BuildObject, "kind" | "digest">[],
        runtimes: readonly Runtime[],
    ): Promise<BuildObject[]> {
        // take every root's builds once, and the builds each runtime imports
        const builds = new Set<Digest>();
        const imported = new Map<Digest, Set<Runtime>>();
        for (const root of roots) {
            const image = root.kind === "image" ? await store.image(root.digest) : undefined;
            for (const build of image === undefined ? [root.digest] : BuildImage.builds(image)) {
                builds.add(build);
            }
            for (const runtime of runtimes) {
                const reached =
                    image === undefined
                        ? [root.digest]
                        : await BuildImage.loads(
                              image,
                              runtime,
                              (build) => store.get(build),
                              "export",
                          );
                for (const build of reached) {
                    imported.set(build, (imported.get(build) ?? new Set()).add(runtime));
                }
            }
        }

        // take what the runtimes importing each build run of it, from its manifest
        const files = new Map<Digest, BuildObject>();
        for (const build of builds) {
            const reader = await store.contents(build);
            for (const file of await reader.runFiles([...(imported.get(build) ?? [])])) {
                files.set(file.digest, { kind: "file", digest: file.digest, size: file.size });
            }
        }

        return [...files.values(), ...(await describeRecords(store, roots, builds))];
    },

    /** Give files attached to builds as attachment entries for a write, after the files. */
    async *attachments(
        attachments: readonly BuildAttachment[],
        crypto: DigestCrypto,
    ): AsyncIterable<TarballEntry> {
        for (const attachment of attachments) {
            const bytes = new TextEncoder().encode(
                JSON.stringify(BuildAttachment.parse(attachment)),
            );
            const digest = await Digest.of(bytes, crypto);
            yield TarballEntry.of(BuildObject.path({ kind: "attachment", digest }), bytes);
        }
    },

    /** Give blobs as file entries for a write. */
    async *files(
        blobs: readonly { readonly digest: Digest; readonly bytes: Uint8Array<ArrayBuffer> }[],
    ): AsyncIterable<TarballEntry> {
        for (const { digest, bytes } of blobs) {
            yield TarballEntry.of(BuildObject.path({ kind: "file", digest }), bytes);
        }
    },

    /** Build an object's tarball entry path from its kind and digest. */
    path(object: Pick<BuildObject, "kind" | "digest">): string {
        return `${object.kind}/${object.digest}`;
    },

    /** Read an object's kind and digest from its tarball entry's path. */
    parse(path: string): Pick<BuildObject, "kind" | "digest"> {
        // read the kind and the digest, refusing anything else
        const [kind, named] = path.split("/");
        const digest = Digest.safeParse(named);
        const isKind =
            kind === "file" || kind === "manifest" || kind === "image" || kind === "attachment";
        if (!isKind || !digest.success) {
            throw BuildError.of(
                "OBJECT_INVALID",
                `object '${path}' is invalid: it must be a kind and a digest`,
            );
        }

        return { kind, digest: digest.data };
    },
};

/** A file attached to a build, as a transfer sends it: the build it describes and the file. */
export const BuildAttachment = schema.object({
    /** The build's manifest digest. */
    subject: Digest,
    /** The attached file, stored before its attachment. */
    file: PackageFile,
});
/** A file attached to a build. */
export type BuildAttachment = schema.Infer<typeof BuildAttachment>;

/** The package a build is of and the releases it requires. */
export type BuildRequirements = Pick<PackageManifest, "package" | "requires">;

/** A cache key's build, and how the cache answered: stored, built now, or joined. */
/** An object with its bytes, as a store packs it. */
interface StoredObject {
    /** What the object is. */
    readonly kind: BuildObject["kind"];
    /** The object's digest. */
    readonly digest: Digest;
    /** The object's bytes. */
    readonly bytes: Uint8Array<ArrayBuffer>;
}

/** The blobs a store stores builds in, and the environment it runs in. */
export interface BuildStoreOptions<StoreEnvironment, Storage extends BlobStorage> {
    /** The partition's blobs, whose database declares the store's tables beside its own. */
    readonly blobs: BlobStore<Storage>;
    /** The crypto digesting files and the clock of leases. */
    readonly environment: StoreEnvironment;
    /** The read-only builds the store reads in place, such as a machine's shipped builds. */
    readonly layer?: BuildLayer;
    /** How long a stored build lasts unless a root or a use retains it, by default the blobs'. */
    readonly leaseMilliseconds?: number;
    /** The stores read through, in order, for builds, images and files this store lacks. */
    readonly sources?: readonly BuildStore[];
    /** Pull a package's build whole with its attachments when a file or its graph is missing. */
    readonly pull?: (packageId: PackageId, build: Digest) => Promise<void>;
}

/** Locate some files: absent unless the store has each, else the layer holding any it lacks. */
type FileLocator = (files: readonly PackageFile[]) => { readonly layer: Digest | null } | undefined;

/** Immutable package builds in a partition's blobs, kept while any of its rows references them. */
export class BuildStore<
    StoreEnvironment extends Pick<Environment, "crypto" | "clock" | "telemetry"> = Pick<
        Environment,
        "crypto" | "clock" | "telemetry"
    >,
    Storage extends BlobStorage = BlobStorage,
> {
    /** The database of a standalone store, such as a toolchain's on its machine's disk. */
    static readonly database = defineDatabase({
        name: "builds",
        tables: [...BUILD_STORE_TABLES, ...BLOB_TABLES],
    });

    /** The partition's blobs: the builds' files and manifests. */
    readonly blobs: BlobStore<Storage>;
    /** The crypto and clock the store digests and expires with. */
    readonly environment: StoreEnvironment;
    /** The parsed lists of the builds it opens, shared by their readers within a byte budget. */
    readonly lists = new BuildListCache();
    /** The manifest of each build its users read, parsed once, by digest. */
    readonly #manifests = new Map<Digest, Promise<PackageManifest>>();
    /** The source keeping each build of an image read through it, which keeps the image whole. */
    readonly #located = new Map<Digest, BuildStore>();
    /** The images read so far, by digest, as images never change. */
    readonly #images = new Map<Digest, Promise<BuildImage>>();
    /** What the builds of the images read so far require, by image, as images never change. */
    readonly #requirements = new Map<Digest, Promise<readonly BuildRequirements[]>>();
    /** The builds in flight to the store, by manifest digest, each stored once. */
    readonly #putting = new Map<Digest, Promise<Digest>>();
    /** The read-only builds the store reads in place. */
    readonly layer: BuildLayer;
    /** How long a stored build lasts before a root or a use retains it, in milliseconds. */
    readonly leaseMilliseconds: number;
    /** The stores read through, in order, for builds, images and files this store lacks. */
    readonly sources: readonly BuildStore[];
    /** Pull a package's build whole when a part is missing, absent where every build is whole. */
    readonly #pullRemote: BuildStoreOptions<StoreEnvironment, Storage>["pull"];
    /** The spans and durations of the store's verbs. */
    readonly scope: TelemetryScope;
    /** The cache of builds and derived files by key, retaining what each entry references. */
    readonly cache: BuildCache;

    /** Store builds in a partition's blobs, reading some read-only builds in place. */
    constructor(options: BuildStoreOptions<StoreEnvironment, Storage>) {
        // take the blobs, the environment, the layer and the lease
        this.blobs = options.blobs;
        this.environment = options.environment;
        this.layer = options.layer ?? BuildLayer.empty();
        this.leaseMilliseconds = options.leaseMilliseconds ?? options.blobs.leaseMilliseconds;
        this.sources = options.sources ?? [];
        this.#pullRemote = options.pull;
        this.scope = TelemetryScope.of(import.meta.destack.package, options.environment.telemetry);
        this.cache = new BuildCache(this);
    }

    /** Read when a lease taken now ends. */
    get #leasedUntil(): number {
        return this.environment.clock.now() + this.leaseMilliseconds;
    }

    /** The partition's database with the store's index. */
    get database(): DatabaseConnection {
        return this.blobs.database;
    }

    /** Serve the store's manifests, files and archives below a mount, as a host allows each. */
    fetch(request: Request, mount: URL, access: PackageAccess): Promise<Response> {
        return serveStore(this, mount, request, access);
    }

    /** Identify a build by the digest of its manifest, as a store stores it. */
    static async digest(manifest: PackageManifest, crypto: DigestCrypto): Promise<Digest> {
        const bytes = PackageManifest.serialize(manifest);

        return Digest.of(bytes, crypto);
    }

    /** Store each missing file of a build, then its manifest, returning the manifest's digest. */
    put(source: BuildReader, signal?: AbortSignal): Promise<Digest> {
        return this.scope.span("destack.build.put", {}, () =>
            this.store(
                source,
                async (files) => {
                    await this.blobs.put(
                        files.map((file) => ({
                            digest: file.digest,
                            size: file.size,
                            open: () => source.stream(file.path),
                        })),
                        this.#leasedUntil,
                    );
                },
                signal,
            ),
        );
    }

    /** Store a build, writing the files the store lacks with a write, joining one in flight. */
    protected async store(
        source: BuildReader,
        write: (files: readonly PackageFile[]) => Promise<void>,
        signal?: AbortSignal,
    ): Promise<Digest> {
        // join the storing of the build in flight, or begin it, forgetting it once it settles
        signal?.throwIfAborted();
        const digest = await source.digest();
        let storing = this.#putting.get(digest);
        if (storing === undefined) {
            storing = this.#storeOnce(source, digest, write);
            this.#putting.set(digest, storing);
            const forget = () => this.#putting.delete(digest);
            void storing.then(forget, forget);
        }

        // wait for it under this caller's own signal, the storing going on for the others
        return await (signal === undefined
            ? storing
            : Promise.race([
                  storing,
                  waitAborted(signal).then(() => {
                      throw signal.reason;
                  }),
              ]));
    }

    /** Store a build unless the store has it, under no single caller's signal. */
    async #storeOnce(
        source: BuildReader,
        digest: Digest,
        write: (files: readonly PackageFile[]) => Promise<void>,
    ): Promise<Digest> {
        return (await this.contains(digest))
            ? digest
            : await this.#storeFiles(
                  source,
                  digest,
                  PackageManifest.serialize(source.manifest),
                  write,
              );
    }

    /** Store a build's missing files, then its manifest and record. */
    async #storeFiles(
        source: BuildReader,
        digest: Digest,
        bytes: Uint8Array<ArrayBuffer>,
        write: (files: readonly PackageFile[]) => Promise<void>,
    ): Promise<Digest> {
        // lease the build's blobs, refusing reserved and duplicate file paths
        const list = await source.references();
        requireDistinctPaths(list);
        const files = distinctFiles(list);
        const leased = await this.blobs.lease(
            [digest, ...files.map((file) => file.digest)],
            this.#leasedUntil,
        );

        // store the missing files at once, then the manifest
        await write(files.filter((file) => !leased.has(file.digest)));
        await this.#write(digest, bytes);

        // record the build with its files
        const referenced = files.map((file) => file.digest);
        await this.database.transaction((transaction) =>
            BuildStore.#recordManifest(transaction, digest, bytes.byteLength, referenced, null),
        );

        return digest;
    }

    /** Report whether the store has a build, recorded once every blob it references is stored. */
    async contains(digest: Digest): Promise<boolean> {
        return (await this.#recorded(digest))?.isComplete === true;
    }

    /** Find the store a root is pulled into for some runtimes: this one, else the first source. */
    async storeOf(
        root: Pick<BuildObject, "kind" | "digest">,
        runtimes: readonly Runtime[] | "all" = "all",
    ): Promise<BuildStore | undefined> {
        // take this store when the root is pulled into it
        if (await this.isPulled(root, runtimes)) {
            return this;
        }

        // ask each source in order
        for (const source of this.sources) {
            const keeper = await source.storeOf(root, runtimes);
            if (keeper !== undefined) {
                return keeper;
            }
        }

        return undefined;
    }

    /** Report whether a root is pulled here: whole, or each build's outputs for some runtimes. */
    async isPulled(
        root: Pick<BuildObject, "kind" | "digest">,
        runtimes: readonly Runtime[] | "all",
    ): Promise<boolean> {
        // take a whole root, and refuse a root the store lacks or one asked whole
        if (await this.contains(root.digest)) {
            return true;
        } else if (runtimes === "all" || (await this.#head(root.digest)) === undefined) {
            return false;
        }

        // require each build's manifest and its outputs for the runtimes, as a pull for them stores
        const builds =
            root.kind === "image"
                ? BuildImage.builds(await this.image(root.digest))
                : [root.digest];
        const held = await Promise.all(
            builds.map(async (build) => {
                // read the build's outputs for the runtimes, and whether each is stored
                const manifest = await this.find(build);
                const outputs = Object.entries(manifest?.outputs ?? {}).filter(([, output]) =>
                    runtimes.includes(output.runtime),
                );
                const stored = await Promise.all(
                    outputs.map(([name]) => this.containsOutput(build, name)),
                );

                return manifest !== undefined && stored.every(Boolean);
            }),
        );

        return held.every(Boolean);
    }

    /** Report whether a blob lies here or in a source. */
    async #isReadable(digest: Digest): Promise<boolean> {
        const isHere = (await this.locate([digest])).has(digest);

        return isHere || (await this.#sourceOf(digest)) !== undefined;
    }

    /** Find the first source storing a blob this store lacks, through its own sources too. */
    async #sourceOf(digest: Digest): Promise<BuildStore | undefined> {
        for (const source of this.sources) {
            const [isStored] = await source.has([digest]);
            const storing = isStored === true ? source : await source.#sourceOf(digest);
            if (storing !== undefined) {
                return storing;
            }
        }

        return undefined;
    }

    /** Report whether the store has an output of a build: the whole build, or that output. */
    async containsOutput(digest: Digest, output: string): Promise<boolean> {
        // take a whole build as containing each output
        if (await this.contains(digest)) {
            return true;
        }

        // find the output a pull stored
        const [row] = await this.database
            .select({ name: buildOutput.name })
            .from(buildOutput)
            .where(
                and(
                    eq(buildOutput.manifest, digest),
                    eq(buildOutput.name, output),
                    this.#isLayerMounted(buildOutput.layer),
                ),
            )
            .limit(1);

        return row !== undefined;
    }

    /** Tell which of some blobs the store has, one flag per digest, its layer's among them. */
    async has(digests: readonly Digest[]): Promise<boolean[]> {
        // locate the bytes, and read the records of the manifests and images among them
        const located = await this.locate(digests);
        const stored = digests.filter((digest) => located.has(digest));
        const records =
            stored.length === 0
                ? []
                : await this.database
                      .select({
                          digest: buildManifest.digest,
                          isComplete: buildManifest.isComplete,
                          layer: buildManifest.layer,
                      })
                      .from(buildManifest)
                      .where(inArray(buildManifest.digest, stored));
        const incomplete = new Set(
            records
                .filter((record) => !record.isComplete || !this.#isMounted(record.layer))
                .map((record) => record.digest),
        );

        // have an object whose bytes lie here, unless it is a manifest or image recorded incomplete, which a push writes again
        return digests.map((digest) => located.has(digest) && !incomplete.has(digest));
    }

    /** Read objects as tarball entries, in the order given. */
    async *read(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
    ): AsyncIterable<TarballEntry> {
        // find where the files lie at once
        const located = await this.locate(
            objects.filter((object) => object.kind === "file").map((object) => object.digest),
        );
        let run: { kind: BlobLocation["kind"]; digests: Digest[] } = {
            kind: "packed",
            digests: [],
        };
        const flush = () =>
            run.kind === "packed" ? this.#batch(run.digests) : this.#layered(run.digests);
        for (const object of objects) {
            const location = object.kind === "file" ? located.get(object.digest) : undefined;

            // gather a file with its neighbours that lie alike, read in one run
            if (location !== undefined) {
                if (run.kind !== location.kind) {
                    yield* flush();
                    run = { kind: location.kind, digests: [] };
                }
                run.digests.push(object.digest);
                continue;
            }
            yield* flush();
            run = { kind: "packed", digests: [] };

            // refuse a missing file
            if (object.kind === "file") {
                throw BuildError.of("BLOB_NOT_FOUND", `package file '${object.digest}' not found`);
            }

            // stream a manifest or image as recorded
            const head = await this.head(object.digest);
            yield { path: BuildObject.path(object), size: head.size, body: this.#stream(head) };
        }
        yield* flush();
    }

    /** Read layered files in order as tarball entries. */
    async *#layered(digests: readonly Digest[]): AsyncIterable<TarballEntry> {
        const files = digests.map((digest) => ({ kind: "file" as const, digest }));
        for await (const { kind, digest, bytes } of this.#readLayered(files)) {
            const body = new ReadableStream<Uint8Array>({
                start(controller) {
                    controller.enqueue(bytes);
                    controller.close();
                },
            });
            yield { path: BuildObject.path({ kind, digest }), size: bytes.byteLength, body };
        }
    }

    /** Read layered objects one after another, as small file reads run fastest in sequence. */
    async *#readLayered(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
    ): AsyncIterable<StoredObject> {
        for (const object of objects) {
            const bytes = await present(
                this.layer.read(object.digest),
                `the layered blob ${object.digest}`,
            );
            yield { kind: object.kind, digest: object.digest, bytes };
        }
    }

    /** Read packed files in one batch of their blobs, as tarball entries in order. */
    async *#batch(digests: readonly Digest[]): AsyncIterable<TarballEntry> {
        for await (const { digest, size, body } of this.blobs.batch(digests)) {
            yield { path: BuildObject.path({ kind: "file", digest }), size, body };
        }
    }

    /** Write objects files first, recording the manifests and images of each pack once stored. */
    write(entries: AsyncIterable<TarballEntry>): Promise<void> {
        return this.scope.span("destack.build.write", {}, () =>
            this.#pack(BuildStore.#parsed(entries), "verify"),
        );
    }

    /** Store objects in packs up to the write limit, then record their manifests and images. */
    async #pack(objects: AsyncIterable<StoredObject>, digests: "verify" | "trust"): Promise<void> {
        // gather objects into packs of at most the write limit, noting the builds the write records
        let gathered: StoredObject[] = [];
        let size = 0;
        const recorded = new Set<Digest>();
        const flush = async () => {
            // store the pack, verifying digests from outside, then record its manifests and images from their bytes
            if (digests === "verify") {
                await this.#verify(gathered);
            }
            const attachments = gathered.filter((object) => object.kind === "attachment");
            await this.#put(gathered.filter((object) => object.kind !== "attachment"));
            const buffers = new Map(gathered.map(({ digest, bytes }) => [digest, bytes]));
            await this.#recordObjects(
                gathered.filter((object) => object.kind === "manifest" || object.kind === "image"),
                buffers,
            );
            for (const object of gathered.filter((each) => each.kind === "manifest")) {
                recorded.add(object.digest);
            }

            // attach each build's referred files together, refusing a build another write recorded
            const attached = new Map<Digest, PackageFile[]>();
            for (const object of attachments) {
                const { subject, file } = BuildAttachment.parse(BuildStore.#json(object.bytes));
                if (!recorded.has(subject)) {
                    throw BuildError.of(
                        "OBJECT_INVALID",
                        `attachment '${file.path}' of build '${subject}' is invalid: it must arrive with its build`,
                    );
                }
                attached.set(subject, [...(attached.get(subject) ?? []), file]);
            }
            for (const [subject, files] of attached) {
                await this.attach(subject, files);
            }
            gathered = [];
            size = 0;
        };
        for await (const object of objects) {
            // write the pack before an object that would take it past the limit, then gather it
            if (gathered.length > 0 && size + object.bytes.byteLength > MAX_WRITE_BYTES) {
                await flush();
            }
            gathered.push(object);
            size += object.bytes.byteLength;
        }
        await flush();
    }

    /** Read tarball entries as objects, refusing one past the write limit. */
    static async *#parsed(entries: AsyncIterable<TarballEntry>): AsyncIterable<StoredObject> {
        for await (const entry of entries) {
            // refuse an object past the limit
            const object = BuildObject.parse(entry.path);
            if (entry.size > MAX_WRITE_BYTES) {
                throw BuildError.of(
                    "OBJECT_INVALID",
                    `object '${object.digest}' is invalid: it exceeds ${MAX_WRITE_BYTES} bytes`,
                );
            }

            // read its bytes
            yield {
                kind: object.kind,
                digest: object.digest,
                bytes: await TarballEntry.bytes(entry),
            };
        }
    }

    /** Pull an image's or build's objects for some runtimes from another store, per round. */
    pull(
        source: Pick<BuildStore, "read">,
        root: Pick<BuildObject, "kind" | "digest">,
        runtimes: readonly Runtime[] | "all",
    ): Promise<void> {
        return this.scope.span("destack.build.pull", {}, () => this.#pull(source, root, runtimes));
    }

    /** Read the image, the builds and their lists a round each, then write what runtimes need. */
    async #pull(
        source: Pick<BuildStore, "read">,
        root: Pick<BuildObject, "kind" | "digest">,
        runtimes: readonly Runtime[] | "all",
    ): Promise<void> {
        // pull nothing for a root the store has with every build whole
        if ((await this.#head(root.digest)) !== undefined) {
            const builds =
                root.kind === "image"
                    ? BuildImage.builds(await this.image(root.digest))
                    : [root.digest];

            // trust the layer for what runtimes run, which is all it ships of a build
            const isShipped = (build: Digest) =>
                runtimes !== "all" && this.layer.head(build) !== undefined;
            const stored = builds.filter((build) => !isShipped(build));
            const complete = await this.#complete(
                stored,
                this.#isLayerMounted(buildManifest.layer),
            );
            if (stored.every((build) => complete.has(build))) {
                return;
            }
        }

        // buffer the objects this store lacks, a read per round
        const buffers = new Map<Digest, Uint8Array<ArrayBuffer>>();
        const gather = async (objects: readonly Pick<BuildObject, "kind" | "digest">[]) => {
            for await (const entry of source.read(await this.#lacking(objects))) {
                buffers.set(BuildObject.parse(entry.path).digest, await TarballEntry.bytes(entry));
            }
        };

        // read the image, from its buffer where pulled
        if (root.kind === "image") {
            await gather([root]);
        }
        const bytes = buffers.get(root.digest);
        const image =
            root.kind !== "image"
                ? undefined
                : bytes === undefined
                  ? await this.image(root.digest)
                  : BuildImage.parse(BuildStore.#json(bytes));

        // open each build, a pulled one over its buffered lists
        const builds = image === undefined ? [root.digest] : BuildImage.builds(image);
        await gather(builds.map((build) => ({ kind: "manifest", digest: build })));
        const readers = new Map<Digest, BuildReader>();
        for (const build of builds) {
            readers.set(build, await this.#pulledReader(build, buffers, readers));
        }

        // pull the lists, refusing a pulled build whose lists lack their formats
        const lists = [...readers.values()].flatMap((reader) => reader.lists());
        await gather(lists.map((file) => ({ kind: "file", digest: file.digest })));
        for (const [build, reader] of readers) {
            if (buffers.has(build)) {
                await reader.check();
            }
        }

        // list the objects the runtimes need, the buffered manifests and image among them
        const view = {
            image: async () => present(image, `the image ${root.digest}`),
            contents: async (build: Digest) => found(readers, build),
            references: async (build: Digest) =>
                (await found(readers, build).references()).map((file): BuildObject => ({
                    kind: "file",
                    digest: file.digest,
                    size: file.size,
                })),
            head: async (object: Digest) => {
                const buffer = buffers.get(object);

                return buffer === undefined
                    ? await this.head(object)
                    : BuildStore.#manifestFile(object, buffer.byteLength);
            },
            get: async (build: Digest) => found(readers, build).manifest,
        };
        const objects =
            runtimes === "all"
                ? await BuildObject.of(view, [root])
                : await BuildObject.runtime(view, [root], runtimes);

        // write the files neither stored nor buffered, then the buffered objects
        const stored = await this.has(objects.map((object) => object.digest));
        const files = objects.filter(
            (object, index) =>
                object.kind === "file" && stored[index] !== true && !buffers.has(object.digest),
        );
        await this.write(
            (async function* () {
                yield* source.read(files);
                for (const object of objects.filter((each) => buffers.has(each.digest))) {
                    yield TarballEntry.of(BuildObject.path(object), found(buffers, object.digest));
                }
            })(),
        );
    }

    /** List the objects the store lacks: unstored files, unrecorded manifests and images. */
    async #lacking(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
    ): Promise<Pick<BuildObject, "kind" | "digest">[]> {
        const stored = await this.has(objects.map((object) => object.digest));
        const isHere = await Promise.all(
            objects.map(async (object, index) =>
                object.kind === "file"
                    ? stored[index] === true
                    : (await this.#head(object.digest)) !== undefined,
            ),
        );

        return objects.filter((_object, index) => isHere[index] !== true);
    }

    /** Open a pulled build over its buffered lists, else the stored build. */
    async #pulledReader(
        build: Digest,
        buffers: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
        readers: ReadonlyMap<Digest, BuildReader>,
    ): Promise<BuildReader> {
        const bytes = buffers.get(build);
        const stream = async (path: string) => this.#streamList(buffers, readers, build, path);

        return bytes === undefined
            ? await this.contents(build)
            : new BuildReader(
                  PackageManifest.parse(BuildStore.#json(bytes)),
                  stream,
                  this.environment.crypto,
                  this.lists,
              );
    }

    /** Store an image once every build it lists is recorded, returning its digest. */
    putImage(image: BuildImage): Promise<Digest> {
        return this.scope.span("destack.build.put_lock", {}, async () => {
            // write the image's canonical bytes under their digest
            const bytes = BuildImage.serialize(image);
            const digest = await Digest.of(bytes, this.environment.crypto);
            await this.write(
                (async function* () {
                    yield TarballEntry.of(BuildObject.path({ kind: "image", digest }), bytes);
                })(),
            );

            return digest;
        });
    }

    /** List the files a build references from its index: the layer's table, else its rows. */
    async references(build: Digest): Promise<BuildObject[]> {
        // read a layered build's files from the layer's table
        const layered = this.layer.references(build);
        if (layered !== undefined) {
            return layered.map((file) => ({ kind: "file", ...file }));
        }

        // read a build a source keeps from the source
        const placed = await this.#place(build);
        if (placed === undefined) {
            throw BuildError.of("MANIFEST_NOT_FOUND", `package manifest '${build}' not found`);
        } else if (placed.source !== undefined) {
            return await placed.source.references(build);
        }

        // read the store's rows of the build, each with the size its blob has where it lies
        const rows = await this.database
            .select({ digest: buildReference.digest })
            .from(buildReference)
            .where(eq(buildReference.manifest, build));
        const located = await this.locate(rows.map((row) => row.digest));

        return rows.map((row) => ({
            kind: "file",
            digest: row.digest,
            size: present(located.get(row.digest), `the blob '${row.digest}' of '${build}'`).size,
        }));
    }

    /** List the manifests and images referencing a blob or build, in the layer and sources too. */
    async referrers(digest: Digest): Promise<Digest[]> {
        // read the recorded references, the layer's images and each source's
        const rows = await this.database
            .select({ manifest: buildReference.manifest })
            .from(buildReference)
            .where(eq(buildReference.digest, digest));
        const sourced = await Promise.all(this.sources.map((source) => source.referrers(digest)));

        return [
            ...new Set([
                ...rows.map((row) => row.manifest),
                ...this.layer.referrers(digest),
                ...sourced.flat(),
            ]),
        ];
    }

    /** Read a stored image's blob once, as images never change, recorded or not. */
    image(digest: Digest): Promise<BuildImage> {
        // read the image's bytes, forgetting a failed read
        const known = this.#images.get(digest);
        if (known !== undefined) {
            return known;
        }
        const reading = (async () => {
            // read an image a source has in place
            const location = (await this.locate([digest])).get(digest);
            if (location === undefined) {
                // place the image's builds in its source, which keeps it whole
                const source = present(await this.#sourceOf(digest), `image '${digest}'`);
                const read = await source.image(digest);
                for (const build of BuildImage.builds(read)) {
                    this.#located.set(build, source);
                }

                return read;
            }

            // read the image's bytes here
            const body = BuildStore.#readable(this.#bytes(digest, location));

            return BuildImage.deserialize(new Uint8Array(await new Response(body).arrayBuffer()));
        })();
        this.#images.set(digest, reading);
        reading.catch(() => this.#images.delete(digest));

        return reading;
    }

    /** Read an image's builds by package name, its root's among them, each with its package. */
    async entriesOf(image: BuildImage): Promise<Record<string, DependencyRelease>> {
        return await BuildImage.entries(image, (digest) => this.get(digest));
    }

    /** Record an image whose blobs the store has, as a moved space's: its builds, then it. */
    async record(digest: Digest): Promise<void> {
        // read the image from the blobs
        const stream = BuildStore.#readable(this.blobs.read(digest));
        const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
        const image = BuildImage.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );

        // record each build, then the image
        const builds = BuildImage.builds(image).map(
            (build) => ({ kind: "manifest", digest: build }) as const,
        );
        await this.#recordObjects([...builds, { kind: "image", digest }], new Map());
    }

    /** Attach stored blobs to a stored build as attachments, refusing another file at a path. */
    async attach(subject: Digest, files: readonly PackageFile[]): Promise<void> {
        // require the build and the blobs
        await this.head(subject);
        await this.#requireStored(subject, files);

        // record the attachments in one transaction, which splits a long insert, keeping those another writer recorded the same
        await this.database.transaction(async (transaction) => {
            await transaction
                .insert(buildAttachment)
                .values(
                    files.map((file) => ({
                        subject,
                        path: file.path,
                        digest: file.digest,
                        size: file.size,
                        mediaType: file.mediaType,
                    })),
                )
                .onConflictDoNothing();
        });

        // refuse a path another writer attached a different file at
        const recorded = new Map(
            (await this.attachments(subject)).map((file) => [file.path, file.digest]),
        );
        const conflict = files.find((file) => recorded.get(file.path) !== file.digest);
        if (conflict !== undefined) {
            throw BuildError.of(
                "ATTACHMENT_CONFLICT",
                `cannot attach attachment '${conflict.path}' to build '${subject}': another file is attached there`,
            );
        }
    }

    /** Store a build's graph: each module's file, all attached together at its module's path. */
    async attachGraph(subject: Digest, modules: readonly graph.Module[]): Promise<void> {
        // encode and store each module's file
        const { crypto } = this.environment;
        const files: PackageFile[] = [];
        for (const module of modules) {
            const { digest, bytes } = await graph.Module.file(module, crypto);
            await this.#write(digest, bytes);
            const path = `${GRAPH_PREFIX}${module.path}`;
            files.push({ path, digest, size: bytes.byteLength, mediaType: "application/json" });
        }

        // attach the files at once, so a build carries its whole graph or none
        await this.attach(subject, files);
    }

    /** Report whether a build carries its graph here or in a source, without pulling it. */
    async isIndexed(subject: Digest): Promise<boolean> {
        return (await this.#attachedGraph(subject)).length > 0;
    }

    /** List a build's graph files, pulling the build with its attachments when it has none here. */
    async #graphFiles(subject: Digest): Promise<PackageFile[]> {
        // take the attached graph, else pull the package's build whole, which brings its attachments
        let attached = await this.#attachedGraph(subject);
        const owner = (await this.get(subject)).package;
        if (attached.length === 0 && this.#pullRemote !== undefined && "id" in owner) {
            await this.#pullRemote(owner.id, subject);
            attached = await this.#attachedGraph(subject);
        }

        // refuse a build no builder attached a graph to
        if (attached.length === 0) {
            throw BuildError.of("GRAPH_NOT_FOUND", `graph of build '${subject}' not found`);
        }

        return attached;
    }

    /** List the graph files attached to a build here or in a source. */
    async #attachedGraph(subject: Digest): Promise<PackageFile[]> {
        const attached = await this.attachments(subject);

        return attached.filter((file) => file.path.startsWith(GRAPH_PREFIX));
    }

    /** Read a build's graph: each module's file attached to it. */
    async graph(subject: Digest): Promise<graph.Module[]> {
        return await Promise.all(
            (await this.#graphFiles(subject)).map(async (file) =>
                graph.Module.parse(JSON.parse(new TextDecoder().decode(await this.file(file)))),
            ),
        );
    }

    /** List a manifest's attachments, else its keeping source's, as OCI's referrers API does. */
    async attachments(subject: Digest): Promise<PackageFile[]> {
        // read the attachments attached here
        const rows = await this.database
            .select({
                path: buildAttachment.path,
                digest: buildAttachment.digest,
                size: buildAttachment.size,
                mediaType: buildAttachment.mediaType,
            })
            .from(buildAttachment)
            .where(eq(buildAttachment.subject, subject));

        // read the attachments the keeping source attached, for a build this store lacks
        if (rows.length === 0 && !(await this.contains(subject))) {
            const source = await this.#sourceOf(subject);

            return (await source?.attachments(subject)) ?? rows;
        }

        return rows.toSorted((left, right) => comparePath(left, right));
    }

    /** Read the file attached to a manifest at a path, absent for a manifest without one. */
    async attachment(subject: Digest, path: string): Promise<PackageFile | undefined> {
        const [row] = await this.database
            .select({
                path: buildAttachment.path,
                digest: buildAttachment.digest,
                size: buildAttachment.size,
                mediaType: buildAttachment.mediaType,
            })
            .from(buildAttachment)
            .where(and(eq(buildAttachment.subject, subject), eq(buildAttachment.path, path)));

        // read the attachment the keeping source attached, for a build this store lacks
        if (row === undefined && !(await this.contains(subject))) {
            const source = await this.#sourceOf(subject);

            return await source?.attachment(subject, path);
        }

        return row;
    }

    /** Read what each build of a stored image requires once, as images never change. */
    requirements(image: Digest): Promise<readonly BuildRequirements[]> {
        // read each image's manifests once, forgetting a failed read
        const known = this.#requirements.get(image);
        if (known !== undefined) {
            return known;
        }
        const reading = (async () => {
            // answer an image a source keeps from the source, which reads it once for every store reading through
            const isHere = (await this.locate([image])).has(image);
            const source = isHere ? undefined : await this.#sourceOf(image);
            if (source !== undefined) {
                return await source.requirements(image);
            }

            // read the manifest of each build the image holds
            const requirements: BuildRequirements[] = [];
            for (const build of BuildImage.builds(await this.image(image))) {
                const { package: built, requires } = await this.get(build);
                requirements.push({ package: built, requires });
            }

            return requirements;
        })();
        this.#requirements.set(image, reading);
        reading.catch(() => this.#requirements.delete(image));

        return reading;
    }

    /** Read a build's manifest, verified and parsed once. */
    async get(digest: Digest): Promise<PackageManifest> {
        return (await this.contents(digest)).manifest;
    }

    /** Open the root build of a stored image. */
    async root(image: Digest): Promise<BuildReader> {
        return await this.contents((await this.image(image)).root);
    }

    /** Read a stored build's manifest, absent for a build the store does not record. */
    async find(digest: Digest): Promise<PackageManifest | undefined> {
        return (await this.#find(digest))?.manifest;
    }

    /** Open a stored build for selective reads, with a reader its caller keeps while it reads. */
    async contents(digest: Digest): Promise<BuildReader> {
        const contents = await this.#find(digest);
        if (contents === undefined) {
            throw BuildError.of("MANIFEST_NOT_FOUND", `package manifest '${digest}' not found`);
        }

        return contents;
    }

    /** Open a stored build for selective reads, absent for a build the store does not record. */
    async #find(digest: Digest): Promise<BuildReader | undefined> {
        // find where the build's manifest lies, forgetting the manifest of a build no longer recorded
        const placed = await this.#place(digest);
        if (placed === undefined) {
            this.#manifests.delete(digest);

            return undefined;
        }

        // read its manifest once, sharing its source's, and forget a failed read
        let manifest = this.#manifests.get(digest);
        if (manifest === undefined) {
            manifest =
                placed.source === undefined
                    ? this.file(placed.head).then((bytes) => PackageManifest.deserialize(bytes))
                    : placed.source.get(digest);
            this.#manifests.set(digest, manifest);
            manifest.catch(() => this.#manifests.delete(digest));
        }

        return this.#open(await manifest, placed.head);
    }

    /** Find where a build's manifest lies: its image's source, else here, else a source. */
    async #place(
        digest: Digest,
    ): Promise<
        { readonly head: PackageFile; readonly source: BuildStore | undefined } | undefined
    > {
        // look in the source keeping the build's image whole, then here, then in the first source with it
        const keeper = this.#located.get(digest);
        const kept = keeper === undefined ? undefined : await keeper.#head(digest);
        const local = kept === undefined ? await this.#head(digest) : undefined;
        const source =
            kept === undefined && local === undefined ? await this.#sourceOf(digest) : undefined;
        const head =
            kept ?? local ?? (source === undefined ? undefined : await source.#head(digest));

        return head === undefined
            ? undefined
            : { head, source: kept === undefined ? source : keeper };
    }

    /** Open a build's reader over its manifest's listed files. */
    #open(manifest: PackageManifest, head: PackageFile): BuildReader {
        const reader: BuildReader = new BuildReader(
            manifest,
            async (path, signal) => {
                // pull a file no store has, then stream it from where it lies
                signal?.throwIfAborted();
                const file = await reader.file(path);
                const owner = manifest.package;
                const isMissing = !(await this.#isReadable(file.digest));
                if (this.#pullRemote !== undefined && "id" in owner && isMissing) {
                    await this.#pullRemote(owner.id, head.digest);
                }
                const stream = this.#stream(file);

                return signal ? Pipe.through(stream, new TransformStream(), signal) : stream;
            },
            this.environment.crypto,
            this.lists,
        );

        return reader;
    }

    /** Lease a build or file until a moment, retaining it and what it references until then. */
    async lease(digest: Digest, expiresAt: Date): Promise<void> {
        await this.blobs.lease([digest], expiresAt.getTime());
    }

    /** Retain builds and images under a root's name, such as a space's, until it retains others. */
    async retain(name: string, roots: ReadonlySet<Digest>): Promise<void> {
        // find the roots the layer holds
        const layered = [...roots].flatMap((digest) => {
            const object = this.layer.object(digest);

            return object === undefined ? [] : [object];
        });

        // copy them up with each image's builds, so they outlive the layer
        const builds = layered.flatMap((object) => {
            const image = object.kind === "image" ? this.layer.image(object.digest) : undefined;

            return (image === undefined ? [] : BuildImage.builds(image)).map((digest) => ({
                kind: "manifest" as const,
                digest,
            }));
        });
        if (layered.length > 0) {
            await this.copyUp([...layered, ...builds]);
        }

        // record the roots
        await this.database.transaction((transaction) =>
            BuildStore.replaceRoot(transaction, name, roots),
        );
    }

    /** Forget a root, leaving what only it retained to a collection. */
    async forget(name: string): Promise<void> {
        await this.retain(name, new Set());
    }

    /** Copy in what builds and images have only through the layer, as overlayfs does. */
    copyUp(roots: readonly Pick<BuildObject, "kind" | "digest">[]): Promise<void> {
        return this.scope.span("destack.build.copy_up", {}, () => this.#copyUp(roots));
    }

    /** Store the layered objects of the roots the store lacks, trusted as the host's own. */
    async #copyUp(roots: readonly Pick<BuildObject, "kind" | "digest">[]): Promise<void> {
        // copy nothing without a layer
        if (this.layer.isEmpty || roots.length === 0) {
            return;
        }

        // list each build's objects and each image's own record, of the roots the store has
        const unique = [...new Map(roots.map((root) => [root.digest, root])).values()];
        const heads = await Promise.all(
            unique.map(async (root) => ({ ...root, head: await this.#head(root.digest) })),
        );
        const listed = heads.filter((root) => root.kind === "manifest" && root.head !== undefined);
        const complete = await this.#complete(
            listed.map((root) => root.digest),
            isNull(buildManifest.layer),
        );
        const builds = listed.filter((root) => !complete.has(root.digest));
        const images = heads.flatMap((root): BuildObject[] =>
            root.kind === "image" && root.head !== undefined
                ? [{ kind: "image", digest: root.digest, size: root.head.size }]
                : [],
        );
        const objects = [...(await BuildObject.of(this, builds)), ...images];

        // find the layered objects the store lacks: files without a pack, records not complete
        const layered = objects.filter((object) =>
            object.kind === "file"
                ? this.layer.size(object.digest) !== undefined
                : this.layer.head(object.digest) !== undefined,
        );
        const files = layered
            .filter((object) => object.kind === "file")
            .map((object) => object.digest);
        const packed = new Set((await this.blobs.placements(files)).map(({ digest }) => digest));
        const records = layered
            .filter((object) => object.kind !== "file")
            .map((object) => object.digest);
        const rows = await this.#complete(records, isNull(buildManifest.layer));
        const copied = layered.filter((object) =>
            object.kind === "file" ? !packed.has(object.digest) : !rows.has(object.digest),
        );

        // store the files from the layer, each read as its pack reaches it
        await this.blobs.put(
            copied
                .filter((object) => object.kind === "file")
                .map(({ digest }) => ({
                    digest,
                    size: present(this.layer.size(digest), `the layered blob ${digest}`),
                    open: async () => {
                        const bytes = await present(
                            this.layer.read(digest),
                            `the layered blob ${digest}`,
                        );

                        return BuildStore.#chunks(bytes);
                    },
                })),
            this.#leasedUntil,
        );

        // record the manifests and images from the layer, trusted as the host's own
        const recorded = copied.filter((object) => object.kind !== "file");
        await this.#pack(this.#readLayered(recorded), "trust");
    }

    /** Find which of some manifests and images the store records complete, by an accepted row. */
    async #complete(
        digests: readonly Digest[],
        accepted: SQLWrapper,
    ): Promise<ReadonlySet<Digest>> {
        const rows = new Set<Digest>();
        for (let start = 0; start < digests.length; start += BATCH_ROWS) {
            const stored = await this.database
                .select({ digest: buildManifest.digest })
                .from(buildManifest)
                .where(
                    and(
                        inArray(buildManifest.digest, digests.slice(start, start + BATCH_ROWS)),
                        eq(buildManifest.isComplete, true),
                        accepted,
                    ),
                );
            for (const row of stored) {
                rows.add(row.digest);
            }
        }

        return rows;
    }

    /** Read the builds each root retains, by root name, the cache entries' roots left out. */
    async roots(): Promise<ReadonlyMap<string, ReadonlySet<string>>> {
        // group the root rows by name, the cache entries' roots left out
        const rows = await this.database.select().from(buildRoot);
        const roots = new Map<string, Set<string>>();
        for (const row of rows.filter((each) => !each.name.startsWith(CACHE_ROOT_PREFIX))) {
            const retained = roots.get(row.name) ?? new Set<string>();
            retained.add(row.digest);
            roots.set(row.name, retained);
        }

        return roots;
    }

    /** Delete every build's index, as the partition leaves its host. */
    async clear(): Promise<void> {
        await this.database.transaction(async (transaction) => {
            for (const table of [
                buildReference,
                buildAttachment,
                buildRoot,
                buildCache,
                buildManifest,
            ]) {
                await transaction.delete(table);
            }
        });
        this.#manifests.clear();
    }

    /** List the real directories the store's layer keeps files in, which its readers read. */
    layerDirectories(): Promise<readonly string[]> {
        return this.layer.directories();
    }

    /** Clone a layered file to a path from its layer, refusing one it lacks. */
    protected async placeLayered(
        file: PackageFile,
        destination: string,
        files: FileSystem,
    ): Promise<void> {
        const layered = present(this.layer.file(file.digest), `the layered file ${file.path}`);
        await files.copyFile(layered, destination);
    }

    /** Read one complete file and verify its exact bytes. */
    async file(file: PackageFile): Promise<Uint8Array<ArrayBuffer>> {
        // read and verify the file bytes, here or through a source
        const bytes = new Uint8Array(await new Response(this.#stream(file)).arrayBuffer());
        await PackageFile.verify(file, bytes, this.environment.crypto);

        return bytes;
    }

    /** Read many stored files whole from where each lies. */
    async *batch(
        files: readonly PackageFile[],
    ): AsyncIterable<{ readonly file: PackageFile; readonly bytes: Uint8Array<ArrayBuffer> }> {
        const located = await this.locate(files.map((file) => file.digest));
        for (const file of files) {
            const location = located.get(file.digest);
            const body = BuildStore.#readable(
                location === undefined
                    ? this.#slice(file, 0, Number.POSITIVE_INFINITY)
                    : this.#bytes(file.digest, location),
            );
            yield { file, bytes: new Uint8Array(await new Response(body).arrayBuffer()) };
        }
    }

    /** Describe a manifest's or image's file here or in a source, leaving its bytes unread. */
    async head(digest: Digest): Promise<PackageFile> {
        // find the head here, else in the source storing it
        const local = await this.#head(digest);
        const source = local === undefined ? await this.#sourceOf(digest) : undefined;
        const head = local ?? (source === undefined ? undefined : await source.#head(digest));
        if (head === undefined) {
            throw BuildError.of("MANIFEST_NOT_FOUND", `package manifest '${digest}' not found`);
        }

        return head;
    }

    /** Describe a manifest's or image's file, absent for one the store does not record. */
    async #head(digest: Digest): Promise<PackageFile | undefined> {
        const record = await this.#recorded(digest);

        return record === undefined ? undefined : BuildStore.#manifestFile(digest, record.size);
    }

    /** Describe a manifest's or image's file by its digest and size. */
    static #manifestFile(digest: Digest, size: number): PackageFile {
        return { path: MANIFEST_PATH, digest, size, mediaType: "application/json" };
    }

    /** Read the digest of the mounted layer, failing for an empty one, which holds no file. */
    #layerDigest(): Digest {
        return present(this.layer.digest, "the digest of the store's layer");
    }

    /** Report whether a row relies on no layer, or on the one the store mounts. */
    #isMounted(layer: string | null): boolean {
        return layer === null || layer === this.layer.digest;
    }

    /** Match a row relying on no layer, or on the one the store mounts. */
    #isLayerMounted(column: typeof buildManifest.layer | typeof buildOutput.layer) {
        const { digest } = this.layer;

        return digest === undefined ? isNull(column) : or(isNull(column), eq(column, digest));
    }

    /** Read a manifest's or image's record: the layer's, complete, else the store's row. */
    async #recorded(
        digest: Digest,
    ): Promise<{ readonly size: number; readonly isComplete: boolean } | undefined> {
        // take a layered manifest or image, which the layer ships complete
        const layered = this.layer.head(digest);
        if (layered !== undefined) {
            return { size: layered.size, isComplete: true };
        }

        // read the stored row, complete only while the layer it relies on is mounted
        const [stored] = await this.database
            .select({
                size: buildManifest.size,
                isComplete: buildManifest.isComplete,
                layer: buildManifest.layer,
            })
            .from(buildManifest)
            .where(eq(buildManifest.digest, digest));

        return stored === undefined
            ? undefined
            : {
                  size: stored.size,
                  isComplete: stored.isComplete && this.#isMounted(stored.layer),
              };
    }

    /** Find where each blob lies: the layer first, then the store's packs, absent for neither. */
    protected async locate(digests: readonly Digest[]): Promise<Map<Digest, BlobLocation>> {
        // take the layered blobs, then find the others in the packs
        const located = new Map<Digest, BlobLocation>();
        for (const digest of digests) {
            const size = this.layer.size(digest);
            if (size !== undefined) {
                located.set(digest, { kind: "layered", size });
            }
        }
        const packed = digests.filter((digest) => !located.has(digest));
        for (const placement of await this.blobs.placements(packed)) {
            located.set(placement.digest, { kind: "packed", ...placement });
        }

        return located;
    }

    /** Read a range of a located blob: in place from the layer, else from its pack. */
    async *#bytes(
        digest: Digest,
        location: BlobLocation,
        offset = 0,
        length = Number.POSITIVE_INFINITY,
    ): AsyncIterable<Uint8Array> {
        // cut the range from a layered blob's bytes
        if (location.kind === "layered") {
            const bytes = await present(this.layer.read(digest), `the layered blob ${digest}`);
            yield bytes.subarray(offset, Math.min(bytes.byteLength, offset + length));
        }
        // read the range from its pack through the blob store
        else {
            yield* this.blobs.slicePlaced(location, offset, length);
        }
    }

    /** Stream a stored file or manifest, or a range of it. */
    open(
        file: PackageFile,
        range?: { readonly offset: number; readonly length: number },
    ): ReadableStream<Uint8Array> {
        return range === undefined
            ? this.#stream(file)
            : BuildStore.#readable(this.#slice(file, range.offset, range.length));
    }

    /** Stream a blob from where it lies, refusing a missing one by its file's path. */
    #stream(file: PackageFile): ReadableStream<Uint8Array> {
        return BuildStore.#readable(this.#slice(file, 0, Number.POSITIVE_INFINITY));
    }

    /** Read a range of a blob from where it lies, refusing a missing one by its file's path. */
    async *#slice(file: PackageFile, offset: number, length: number): AsyncIterable<Uint8Array> {
        // read a blob stored here
        const location = (await this.locate([file.digest])).get(file.digest);
        if (location !== undefined) {
            yield* this.#bytes(file.digest, location, offset, length);
        }
        // read it through the first source that has it
        else {
            const source = await this.#sourceOf(file.digest);
            if (source === undefined) {
                throw BuildError.of("BLOB_NOT_FOUND", `package file '${file.path}' not found`);
            }
            yield* source.#slice(file, offset, length);
        }
    }

    /** Parse JSON bytes, refusing invalid UTF-8. */
    static #json(bytes: Uint8Array<ArrayBuffer>): unknown {
        return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
    }

    /** Lease and store blobs, refusing bytes of another digest. */
    async #verify(blobs: readonly StoredObject[]): Promise<void> {
        for (const { digest, bytes } of blobs) {
            if ((await Digest.of(bytes, this.environment.crypto)) !== digest) {
                throw BuildError.of(
                    "DIGEST_INVALID",
                    `object is invalid: it does not match its digest '${digest}'`,
                );
            }
        }
    }

    /** Store blobs in one pack, leased until the store's lease ends. */
    async #put(blobs: readonly StoredObject[]): Promise<void> {
        await this.blobs.put(
            blobs.map(({ digest, bytes }) => ({
                digest,
                size: bytes.byteLength,
                open: async () => BuildStore.#chunks(bytes),
            })),
            this.#leasedUntil,
        );
    }

    /** Record stored manifests and images in order, from buffered bytes where given, at once. */
    async #recordObjects(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
        buffers: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
    ): Promise<void> {
        // open each manifest over its buffered lists, else the stored ones
        const readers = new Map<Digest, BuildReader>();
        const sizes = new Map<Digest, number>();
        for (const { digest } of objects.filter((object) => object.kind === "manifest")) {
            const bytes = await this.#bytesOf(digest, buffers);
            const manifest = PackageManifest.parse(BuildStore.#json(bytes));
            const stream = async (path: string) => this.#streamList(buffers, readers, digest, path);
            readers.set(
                digest,
                new BuildReader(manifest, stream, this.environment.crypto, this.lists),
            );
            sizes.set(digest, bytes.byteLength);
        }

        // refuse a build whose lists the store lacks
        const lists = [...readers].flatMap(([digest, reader]) =>
            reader.lists().map((file) => ({ digest, file })),
        );
        const isListed = await this.has(lists.map(({ file }) => file.digest));
        const unlisted = lists.find((_list, index) => isListed[index] !== true);
        if (unlisted !== undefined) {
            throw BuildError.of(
                "MANIFEST_BLOB_INVALID",
                `build '${unlisted.digest}' is invalid: it lists '${unlisted.file.path}', which was not uploaded`,
            );
        }

        // list each build's files and outputs, refusing a list without its format
        const builds = new Map<Digest, RecordedBuild>();
        for (const [digest, reader] of readers) {
            // refuse duplicate paths
            await reader.check();
            const distributed = await reader.references();
            requireDistinctPaths(distributed);

            // list the files each output runs
            const outputs = await Promise.all(
                Object.entries(reader.manifest.outputs).map(async ([name, output]) => ({
                    name,
                    files: await reader.runFiles([output.runtime]),
                })),
            );
            const size = found(sizes, digest);
            builds.set(digest, { size, files: distinctFiles(distributed), outputs });
        }

        // find which of every listed file the store holds in its packs, at once, and which its layer holds
        const listed = [...builds.values()].flatMap((build) => [
            ...build.files,
            ...build.outputs.flatMap((output) => output.files),
        ]);
        const digests = [...new Set(listed.map((file) => file.digest))];
        const packed = new Set((await this.blobs.placements(digests)).map(({ digest }) => digest));
        const isLayered = (file: PackageFile) => this.layer.size(file.digest) !== undefined;
        const locate: FileLocator = (files) =>
            files.every((file) => packed.has(file.digest) || isLayered(file))
                ? {
                      layer: files.every((file) => packed.has(file.digest))
                          ? null
                          : this.#layerDigest(),
                  }
                : undefined;

        // read each image
        const images: {
            readonly digest: Digest;
            readonly size: number;
            readonly image: BuildImage;
        }[] = [];
        for (const { digest } of objects.filter((object) => object.kind === "image")) {
            const bytes = await this.#bytesOf(digest, buffers);
            images.push({
                digest,
                size: bytes.byteLength,
                image: BuildImage.parse(BuildStore.#json(bytes)),
            });
        }

        // refuse an image listing a build recorded neither here nor before
        const outside = images
            .flatMap(({ image }) => BuildImage.builds(image))
            .filter((build) => !builds.has(build));
        const recorded = await this.#recordedBuilds([...new Set(outside)]);
        for (const { digest, image } of images) {
            const absent = BuildImage.builds(image).find(
                (build) => !builds.has(build) && !recorded.has(build),
            );
            if (absent !== undefined) {
                throw BuildError.of(
                    "MANIFEST_NOT_FOUND",
                    `build '${absent}' of image '${digest}' not found`,
                );
            }
        }

        // record each build whole, else each output whose files are all here, then each image
        await this.database.transaction(async (transaction) => {
            for (const [digest, build] of builds) {
                await BuildStore.#recordBuild(
                    transaction,
                    digest,
                    build,
                    locate,
                    found(readers, digest),
                );
            }
            for (const { digest, size, image } of images) {
                // record the image over the layer once one of its builds relies on it
                const isRelying = BuildImage.builds(image).some((build) => {
                    const written = builds.get(build);
                    const relied =
                        written === undefined ? recorded.get(build) : locate(written.files)?.layer;

                    return relied !== undefined && relied !== null;
                });
                await BuildStore.#recordManifest(
                    transaction,
                    digest,
                    size,
                    BuildImage.builds(image),
                    isRelying ? this.#layerDigest() : null,
                );
            }
        });
    }

    /** Record a build in a transaction: whole once every file is here, else each output that is. */
    static async #recordBuild(
        transaction: DatabaseConnection,
        digest: Digest,
        build: RecordedBuild,
        locate: FileLocator,
        reader: BuildReader,
    ): Promise<void> {
        // record it whole with its files, and the layer holding the ones the store lacks
        const { size } = build;
        const whole = locate(build.files);
        if (whole !== undefined) {
            const referenced = build.files.map((file) => file.digest);
            await BuildStore.#recordManifest(transaction, digest, size, referenced, whole.layer);
        }
        // record its lists, then each output whose files are all here
        else {
            const lists = reader.lists().map((file) => file.digest);
            await BuildStore.#reference(transaction, digest, lists);
            await transaction
                .insert(buildManifest)
                .values({ digest, size, isComplete: false })
                .onConflictDoNothing();
            for (const output of build.outputs) {
                // record an output whose files are here, with the layer holding the ones the store lacks
                const located = locate(output.files);
                if (located === undefined) {
                    continue;
                }
                const referenced = [...new Set(output.files.map((file) => file.digest))];
                await BuildStore.#reference(transaction, digest, referenced);
                await transaction
                    .insert(buildOutput)
                    .values({ manifest: digest, name: output.name, layer: located.layer })
                    .onConflictDoUpdate({
                        target: [buildOutput.manifest, buildOutput.name],
                        set: { layer: located.layer },
                    });
            }
        }
    }

    /** Find which builds the store records, each with the layer it relies on, if any. */
    async #recordedBuilds(digests: readonly Digest[]): Promise<Map<Digest, string | null>> {
        // take the layered builds, which rely on the mounted layer
        const recorded = new Map<Digest, string | null>(
            digests
                .filter((digest) => this.layer.head(digest) !== undefined)
                .map((digest) => [digest, this.#layerDigest()]),
        );

        // read the others' rows a batch at a time, each with the layer it relies on
        const others = digests.filter((digest) => !recorded.has(digest));
        for (let start = 0; start < others.length; start += BATCH_ROWS) {
            const rows = await this.database
                .select({ digest: buildManifest.digest, layer: buildManifest.layer })
                .from(buildManifest)
                .where(inArray(buildManifest.digest, others.slice(start, start + BATCH_ROWS)));
            for (const row of rows) {
                recorded.set(row.digest, row.layer);
            }
        }

        return recorded;
    }

    /** Read a stored object's bytes, buffered ones first. */
    async #bytesOf(
        digest: Digest,
        buffers: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
    ): Promise<Uint8Array<ArrayBuffer>> {
        const bytes = buffers.get(digest);

        return (
            bytes ??
            new Uint8Array(
                await new Response(BuildStore.#readable(this.blobs.read(digest))).arrayBuffer(),
            )
        );
    }

    /** Stream a list of a pulled build from the buffers, else from this store. */
    async #streamList(
        buffers: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
        readers: ReadonlyMap<Digest, BuildReader>,
        build: Digest,
        path: string,
    ): Promise<ReadableStream<Uint8Array>> {
        const file = present(
            found(readers, build)
                .lists()
                .find((each) => each.path === path),
            `the list ${path} of build ${build}`,
        );
        const bytes = buffers.get(file.digest);

        return bytes === undefined ? this.#stream(file) : new Blob([bytes]).stream();
    }

    /** Store bytes under their digest unless stored, leased for the row recording them. */
    async #write(digest: Digest, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        await this.blobs.put(
            [{ digest, size: bytes.byteLength, open: async () => BuildStore.#chunks(bytes) }],
            this.#leasedUntil,
        );
    }

    /** Require the store to have each file a build's manifest lists. */
    async #requireStored(digest: string, files: readonly PackageFile[]): Promise<void> {
        // find the files the store lacks, refusing the first
        const stored = await this.has(files.map((file) => file.digest));
        const absent = files.find((_file, index) => stored[index] !== true);
        if (absent !== undefined) {
            throw BuildError.of(
                "MANIFEST_BLOB_INVALID",
                `build '${digest}' is invalid: it lists '${absent.path}', which was not uploaded`,
            );
        }
    }

    /** Record a whole manifest or image in a transaction: its distinct references, then it. */
    static async #recordManifest(
        transaction: DatabaseConnection,
        digest: Digest,
        size: number,
        referenced: readonly Digest[],
        layer: Digest | null,
    ): Promise<void> {
        await BuildStore.#reference(transaction, digest, referenced);
        await transaction
            .insert(buildManifest)
            .values({ digest, size, isComplete: true, layer })
            .onConflictDoUpdate({ target: buildManifest.digest, set: { isComplete: true, layer } });
    }

    /** Record the blobs a build references, a batch of rows at a time. */
    static async #reference(
        transaction: DatabaseConnection,
        manifest: Digest,
        referenced: readonly Digest[],
    ): Promise<void> {
        for (let start = 0; start < referenced.length; start += BATCH_ROWS) {
            const batch = referenced.slice(start, start + BATCH_ROWS);
            await transaction
                .insert(buildReference)
                .values(batch.map((digest) => ({ manifest, digest })))
                .onConflictDoNothing();
        }
    }

    /** Replace the blobs a root retains. */
    static async replaceRoot(
        transaction: DatabaseConnection,
        name: string,
        digests: ReadonlySet<Digest>,
    ): Promise<void> {
        // read the root's blobs
        const rows = await transaction
            .select({ digest: buildRoot.digest })
            .from(buildRoot)
            .where(eq(buildRoot.name, name));
        const current = new Set(rows.map(({ digest }) => digest));

        // add the new ones and drop the old ones
        const added = [...digests].filter((digest) => !current.has(digest));
        const dropped = [...current].filter((digest) => !digests.has(digest));
        for (let start = 0; start < added.length; start += BATCH_ROWS) {
            const batch = added.slice(start, start + BATCH_ROWS);
            await transaction.insert(buildRoot).values(batch.map((digest) => ({ name, digest })));
        }
        for (let start = 0; start < dropped.length; start += BATCH_ROWS) {
            const batch = dropped.slice(start, start + BATCH_ROWS);
            await transaction
                .delete(buildRoot)
                .where(and(eq(buildRoot.name, name), inArray(buildRoot.digest, batch)));
        }
    }

    /** Stream the chunks of an async iterable, reading one per pull. */
    static #readable(chunks: AsyncIterable<Uint8Array>): ReadableStream<Uint8Array> {
        const iterator = chunks[Symbol.asyncIterator]();

        return new ReadableStream({
            async pull(controller) {
                // enqueue the next chunk, or close after the last
                const next = await iterator.next();
                if (next.done === true) {
                    controller.close();
                } else {
                    controller.enqueue(next.value);
                }
            },
            async cancel() {
                await iterator.return?.();
            },
        });
    }

    /** Read some bytes as one chunk. */
    static async *#chunks(bytes: Uint8Array<ArrayBuffer>): AsyncIterable<Uint8Array> {
        yield bytes;
    }
}

/** A build being recorded: its manifest's size, its distinct files, and each output's run files. */
interface RecordedBuild {
    /** The manifest's size in bytes. */
    readonly size: number;
    /** The distinct files it distributes. */
    readonly files: readonly PackageFile[];
    /** The files each output runs, by name. */
    readonly outputs: readonly { readonly name: string; readonly files: readonly PackageFile[] }[];
}

/** Take the first file of each digest. */
function distinctFiles(list: readonly PackageFile[]): PackageFile[] {
    return [...new Map(list.map((file) => [file.digest, file])).values()];
}

/** Refuse a file list with the reserved manifest path or one path twice. */
function requireDistinctPaths(list: readonly PackageFile[]): void {
    const paths = new Set<string>();
    for (const file of list) {
        if (file.path === MANIFEST_PATH || file.path.startsWith(`${MANIFEST_PATH}/`)) {
            throw BuildError.of(
                "MANIFEST_INVALID",
                `build path '${file.path}' is invalid: it is reserved`,
            );
        }
        if (paths.has(file.path)) {
            throw BuildError.of(
                "MANIFEST_INVALID",
                `package file '${file.path}' is invalid: it is a duplicate`,
            );
        }
        paths.add(file.path);
    }
}
