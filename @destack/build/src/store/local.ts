import { blobTable, inArray } from "@destack/db";
import { dirname, join } from "node:path";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore } from "@destack/db/database";
import { connectBunSqlite } from "@destack/db/bun";
import { type Environment, type FileSystem, FileSystemError } from "@destack/environment";
import { type PackageFile, PackagePath } from "@destack/package/file";
import { BATCH_ROWS, BuildObject, BuildStore } from "./store.ts";
import { CACHE_MILLISECONDS, USE_MILLISECONDS } from "../cache/cache.ts";
import { BuildError } from "../error/index.ts";
import {
    BuildReader,
    BuildImage,
    ImportMap,
    LOADED_RUNTIMES,
    MANIFEST_PATH,
    PackageManifest,
    BuildLayer,
    LAYER_BLOBS,
    LAYER_FORMAT,
    LAYER_INDEX,
    LAYER_PACKS,
    LAYER_TREES,
    BuildLayerIndex,
    type BuildLayerFile,
} from "@destack/package/manifest";
import { BuildTree } from "../dependency/tree.ts";
import { Digest, JsonValue, present, schema, Uuid } from "@destack/schema";
import { HOST_RUNTIME, type Runtime } from "@destack/package/runtime";

/** The URL prefix a layer names its builds' modules below. */
const LAYER_BASE = "/";

/** The most bytes of files a pack takes: 16 MiB, below a Worker's 25 MiB per static asset. */
const PACK_BYTES = 16 * 1024 * 1024;

/** The directory below a store's directory with its blobs, by digest. */
const FILES_DIRECTORY = "files";

/** The database file of a standalone store, which indexes its blobs. */
const DATABASE_FILE = "store.db";

/** The directory below a store's directory with the dependency trees of its builds, by manifest. */
const TREES_DIRECTORY = "trees";

/** The directory below a store's directory with each realized tree's directories, by its inputs. */
const REALIZED_DIRECTORY = "realized";

/** The directories of a realized tree, by package name. */
const RealizedTree = schema.record(schema.string(), schema.string());

/** The directory below a store's directory with each process's temporary files, by process. */
const TEMPORARY_DIRECTORY = "tmp";

/** The suffix of the file a process locks while its temporary directory lives. */
const TEMPORARY_LOCK_SUFFIX = ".lock";

/** The directory below a store's directory with the builds its linked layers share, by digest. */
const LAYERED_DIRECTORY = "layered";

/** The directory below a store's directory with its linked layers, by releases and runtimes. */
const LAYERS_DIRECTORY = "layers";

/** The prefix of a directory's root, forgotten once the directory is gone. */
const DIRECTORY_ROOT_PREFIX = "directory:";

/** The files placed at once: well below the default limit of 256 open files on macOS. */
const PLACE_BATCH = 64;

/** The file every process using a store locks shared. */
const STORE_LOCK = "store.lock";

/** The file a store's directory locks while one process opens and collects the store. */
const OPEN_LOCK = "open.lock";

/** The file with the time an idle open last collected the store, in milliseconds. */
const COLLECTED_FILE = "collected";

/** How often an idle open collects the store: daily, as Cargo's gc.auto.frequency defaults to. */
const COLLECT_MILLISECONDS = 24 * 60 * 60_000;

/** The most bytes a local store's blobs take: 10 GiB, three times a platform's 3 GB of builds. */
const MAX_BYTES = 10 * 1024 ** 3;

/** The directory of Destack's caches below the person's cache directory. */
const CACHE_DIRECTORY = "destack";

/** What the person's store has builds for: development tools, or test runs. */
export type LocalStoreUse = "development" | "test";

/** How a layer lays out its builds: linked to its store's pooled builds, or as packs of files. */
export type LayerLayout = "linked" | "packed";

/** The blobs, database, trees directory and environment of a store on this machine's disk. */
export interface LocalBuildStoreOptions {
    /** The blobs, kept in a directory, whose database declares the store's tables. */
    readonly blobs: BlobStore<LocalBlobStorage>;
    /** The directory with the dependency trees. */
    readonly directory: string;
    /** The environment the store runs in. */
    readonly environment: Environment<"files" | "process" | "host">;
    /** The read-only builds the store reads in place, such as a machine's shipped builds. */
    readonly layer?: BuildLayer;
}

/** A file a store writes to a path. */
export interface FilePlacement {
    /** The file. */
    readonly file: PackageFile;
    /** The path it is written to. */
    readonly destination: string;
}

/** A store of builds on this machine's disk, its blobs in a directory, which processes share. */
export class LocalBuildStore
    extends BuildStore<Environment<"files" | "process" | "host">, LocalBlobStorage>
    implements AsyncDisposable
{
    /** The directory with the store's blobs, database and trees. */
    readonly directory: string;
    /** The directory of the dependency trees its builds are materialized in, one per manifest. */
    readonly trees: string;
    /** The directory of the realized trees' records, by the digest of their inputs. */
    readonly realized: string;
    /** The roots being placed now, by path, which the trees placing one at once share. */
    readonly #placing = new Map<string, Promise<void>>();
    /** What disposal releases: the database the store opened, and a temporary store's directory. */
    readonly #disposables: AsyncDisposableStack;
    /** The store that renews the builds' retention, absent when this one does. */
    readonly #maintainer: LocalBuildStore | undefined;
    /** The builds a view used, renewed by its maintainer once the view closes. */
    readonly #used = new Set<Digest>();
    /** The directory this process writes temporary files in, which its views share. */
    readonly #temporary: string;

    /** Store builds in a directory's blobs indexed in a database, and what disposal releases. */
    private constructor(
        options: LocalBuildStoreOptions,
        disposables: AsyncDisposableStack,
        maintainer: LocalBuildStore | undefined,
    ) {
        // store the blobs, the database, the trees, the disposables and the store renewing retention
        super(options);
        this.directory = options.directory;
        this.trees = join(options.directory, TREES_DIRECTORY);
        this.realized = join(options.directory, REALIZED_DIRECTORY);
        this.#temporary =
            maintainer === undefined
                ? join(options.directory, TEMPORARY_DIRECTORY, Uuid.v4(options.environment.random))
                : maintainer.#temporary;
        this.#disposables = disposables;
        this.#maintainer = maintainer;
    }

    /** Store builds in blobs and an opened database, their trees below a directory. */
    static async of(options: LocalBuildStoreOptions): Promise<LocalBuildStore> {
        return await LocalBuildStore.#create(options, new AsyncDisposableStack());
    }

    /** Store a directory's builds, creating the directory of their trees. */
    static async #create(
        options: LocalBuildStoreOptions,
        disposables: AsyncDisposableStack,
        maintainer?: LocalBuildStore,
    ): Promise<LocalBuildStore> {
        // create the trees directory and this process's temporary directory, which a view shares
        const store = new LocalBuildStore(options, disposables, maintainer);
        await options.environment.files.mkdir(store.trees, { recursive: true });
        if (maintainer === undefined) {
            await store.#openTemporary(disposables);
        }

        return store;
    }

    /** Delete ended processes' temporary directories, then lock this process's until disposal. */
    async #openTemporary(disposables: AsyncDisposableStack): Promise<void> {
        // delete what ended processes left
        const { files } = this.environment;
        await files.mkdir(dirname(this.#temporary), { recursive: true });
        await this.#deleteEndedTemporaries();

        // lock this process's temporary directory again should a sweep have deleted its lock first
        const lock = `${this.#temporary}${TEMPORARY_LOCK_SUFFIX}`;
        let held = await files.lock(lock, "exclusive");
        while (!(await files.exists(lock))) {
            await held.close();
            held = await files.lock(lock, "exclusive");
        }

        // create it once locked, removing both on disposal
        await files.mkdir(this.#temporary);
        disposables.defer(async () => {
            await files.remove(this.#temporary, { recursive: true, force: true });
            await files.remove(lock, { force: true });
            await held.close();
        });
    }

    /** Delete the temporary directories of ended processes: unlocked, or without a lock. */
    async #deleteEndedTemporaries(): Promise<void> {
        const { files } = this.environment;
        const temporaries = dirname(this.#temporary);
        for (const name of await LocalBuildStore.#entries(temporaries, files)) {
            // delete a directory without a lock, as a live process locks before creating its directory
            if (!name.endsWith(TEMPORARY_LOCK_SUFFIX)) {
                const lock = join(temporaries, `${name}${TEMPORARY_LOCK_SUFFIX}`);
                if (!(await files.exists(lock))) {
                    await files.remove(join(temporaries, name), { recursive: true, force: true });
                }
            }
            // delete a directory and its lock once no live process holds the lock
            else {
                const ended = await files.tryLock(join(temporaries, name), "exclusive");
                if (ended !== undefined) {
                    const directory = join(
                        temporaries,
                        name.slice(0, -TEMPORARY_LOCK_SUFFIX.length),
                    );
                    await files.remove(directory, { recursive: true, force: true });
                    await files.remove(join(temporaries, name), { force: true });
                    await ended.close();
                }
            }
        }
    }

    /** Open a standalone store in a directory: its blobs, and its database migrated. */
    static async #openDirectory(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
        disposables: AsyncDisposableStack,
        maintainer?: LocalBuildStore,
    ): Promise<LocalBuildStore> {
        // open the blobs, then the database, closed on disposal
        const storage = await LocalBlobStorage.open(join(directory, FILES_DIRECTORY), environment, {
            synchronous: "off",
        });
        const database = await connectBunSqlite(
            join(directory, DATABASE_FILE),
            BuildStore.database,
            { environment },
        );
        disposables.defer(() => database.close());
        await database.migrate(BuildStore.database.tables);
        const blobs = new BlobStore({ storage, database, environment });

        return await LocalBuildStore.#create(
            { blobs, directory, environment },
            disposables,
            maintainer,
        );
    }

    /** View this store's directory through another environment, this store renewing retention. */
    async view(
        environment: Environment<"files" | "process" | "host">,
        scheduler?: Environment["scheduler"],
    ): Promise<LocalBuildStore> {
        // release this store after the view, as one operation of the scheduler's world
        const disposables = new AsyncDisposableStack();
        disposables.defer(() =>
            scheduler === undefined
                ? this.#release(view)
                : scheduler.run(() => this.#release(view)),
        );
        const view = await LocalBuildStore.#openDirectory(
            this.directory,
            environment,
            disposables,
            this,
        );

        return view;
    }

    /** Renew the builds a closed view used, then close this store. */
    async #release(view: LocalBuildStore): Promise<void> {
        for (const digest of view.#used) {
            await this.use([digest]);
        }
        await this[Symbol.asyncDispose]();
    }

    /** Store a build, retaining it a week past this use. */
    override async put(...stored: Parameters<BuildStore["put"]>): Promise<Digest> {
        const digest = await super.put(...stored);
        await this.use([digest]);

        return digest;
    }

    /** Open a stored build, retaining it a week past this use, as a cache does. */
    override async contents(digest: Digest): Promise<BuildReader> {
        await this.use([digest]);

        return super.contents(digest);
    }

    /** Lease stored builds a week on once a day they are used, retaining them and their files. */
    async use(digests: readonly Digest[]): Promise<void> {
        // record the uses for the maintaining store to renew once this view closes
        if (this.#maintainer !== undefined) {
            for (const digest of digests) {
                this.#used.add(digest);
            }

            return;
        }

        // renew each lease a day or more old, a layered build having none
        const now = this.environment.clock.now();
        for (let start = 0; start < digests.length; start += BATCH_ROWS) {
            const batch = digests.slice(start, start + BATCH_ROWS);
            const leased = await this.database
                .select({ digest: blobTable.digest, until: blobTable.leasedUntil })
                .from(blobTable)
                .where(inArray(blobTable.digest, batch));
            for (const { digest, until } of leased) {
                if (until - CACHE_MILLISECONDS <= now - USE_MILLISECONDS) {
                    await this.lease(Digest.parse(digest), new Date(now + CACHE_MILLISECONDS));
                }
            }
        }
    }

    /** Root some builds by a directory, until the directory is gone. */
    async rootDirectory(directory: string, manifests: ReadonlySet<Digest>): Promise<void> {
        await this.retain(`${DIRECTORY_ROOT_PREFIX}${directory}`, manifests);
    }

    /** Create a directory to write in on the blobs' volume, removed when the store closes. */
    async makeTemporaryDirectory(prefix: string): Promise<string> {
        // create a fresh directory below this process's temporary directory
        const { files, random } = this.environment;
        const directory = join(this.#temporary, `${prefix}${Uuid.v4(random)}`);
        await files.mkdir(directory, { recursive: true });

        return directory;
    }

    /** Store a build written in a directory the store consumes, moving its files into the blobs. */
    move(directory: string, signal?: AbortSignal): Promise<Digest> {
        return this.scope.span("destack.build.move", {}, () => this.#move(directory, signal));
    }

    /** Move a written build's missing files into the blobs, then remove what is left. */
    async #move(directory: string, signal: AbortSignal | undefined): Promise<Digest> {
        // store each missing file by moving it in, recorded under a lease
        const { files, crypto, clock } = this.environment;
        const source = await BuildReader.openDirectory(directory, files, crypto, this.lists);
        const digest = await this.store(
            source,
            async (moved) => {
                const placements = [];
                for (const file of moved) {
                    placements.push(
                        await this.blobs.storage.move(join(directory, file.path), file.digest),
                    );
                }
                await this.blobs.record(placements, clock.now() + this.leaseMilliseconds);
            },
            signal,
        );

        // remove what the directory has left
        await files.remove(directory, { recursive: true, force: true });

        return digest;
    }

    /** The store's layered builds of this layer format, by digest, which linked layers share. */
    get #pool(): string {
        return join(this.directory, LAYERED_DIRECTORY, String(LAYER_FORMAT));
    }

    /** Write release images to a layer: whole builds for a computer, run files for the cloud. */
    writeLayer(
        images: readonly Digest[],
        runtimes: readonly Runtime[],
        directory: string,
        layout: LayerLayout,
    ): Promise<void> {
        return this.scope.span("destack.build.write_layer", {}, () =>
            this.#writeLayer(images, runtimes, directory, layout),
        );
    }

    /** Link a path to a target, replacing a link to another, such as an older format's pool. */
    async #link(target: string, path: string): Promise<void> {
        // leave a link to the target, and replace any other
        const { files } = this.environment;
        if (await files.exists(path)) {
            if ((await files.readLink(path)) === target) {
                return;
            }
            await files.remove(path);
        }
        await files.symlink(target, path);
    }

    /** Write the images' builds in a layout: packs of files, or whole builds and run trees. */
    async #writeLayer(
        images: readonly Digest[],
        runtimes: readonly Runtime[],
        directory: string,
        layout: LayerLayout,
    ): Promise<void> {
        // read each image and its builds
        const shipped = new Map<Digest, BuildImage>();
        const builds = new Set<Digest>();
        for (const digest of images) {
            const image = await this.image(digest);
            shipped.set(digest, image);
            for (const build of BuildImage.builds(image)) {
                builds.add(build);
            }
        }

        // write a packed layer's files alone, what the runtimes run, as the cloud only runs
        if (layout === "packed") {
            await this.#writePacked(shipped, runtimes, directory);

            return;
        }

        // write each build whole once in the store's layered builds, and link it from the layer
        const { files } = this.environment;
        const pool = this.#pool;
        await files.mkdir(directory, { recursive: true });
        const readers = new Map<Digest, BuildReader>();
        for (const digest of builds) {
            const reader = await this.contents(digest);
            readers.set(digest, reader);
            await this.#writePooled(reader, join(pool, digest));
            await this.#link(join(pool, digest), join(directory, digest));
        }

        // remove the builds left out
        const kept = new Set<string>(builds);
        const stale = (await files.readDirectory(directory)).filter(
            (entry) =>
                (entry.kind === "directory" || entry.kind === "symlink") &&
                entry.name !== LAYER_TREES &&
                !kept.has(entry.name),
        );
        for (const entry of stale) {
            await files.remove(join(directory, entry.name), { recursive: true });
        }

        // lay out the releases' trees once for the host in the store's own trees, linked from the layer
        const isHosting = runtimes.includes(HOST_RUNTIME);
        const trees = this.trees;
        await files.mkdir(trees, { recursive: true });
        await this.#link(trees, join(directory, LAYER_TREES));
        for (const image of isHosting ? shipped.values() : []) {
            const releases = await BuildImage.releases(image, (digest) => this.get(digest));
            const resolve = (build: Digest, name: string) => BuildImage.resolve(image, build, name);
            await BuildTree.materialize(this, releases, resolve, trees, { kind: "compile" });
        }

        // index each build's manifest and files last
        const sizes: BuildLayerIndex["builds"] = {};
        const placed = new Map<Digest, BuildLayerFile>();
        const listed = new Map<Digest, PackageFile[]>();
        for (const [digest, reader] of readers) {
            sizes[digest] = PackageManifest.serialize(reader.manifest).byteLength;
            const distributed = await reader.references();
            listed.set(digest, distributed);
            for (const file of distributed) {
                if (!placed.has(file.digest)) {
                    placed.set(file.digest, {
                        digest: file.digest,
                        size: file.size,
                        addressing: "path",
                        path: `${digest}/${file.path}`,
                    });
                }
            }
        }
        const linked = [...placed.values()];
        await BuildLayer.writeIndex(directory, shipped, sizes, listed, "path", linked, [], files);
    }

    /** Write a linked layer of release images once, keyed with the runtimes. */
    async writeLinkedLayer(
        images: readonly Digest[],
        runtimes: readonly Runtime[],
    ): Promise<string> {
        // leave a layer written before in this format
        const { files, crypto } = this.environment;
        const key = await Digest.json({ format: LAYER_FORMAT, releases: images, runtimes }, crypto);
        const directory = join(this.directory, LAYERS_DIRECTORY, key);
        if (await files.exists(directory)) {
            return directory;
        }

        // write it in this process's temporary directory, then move it in, leaving one another process moved in first
        const written = await this.makeTemporaryDirectory("layer-");
        await files.mkdir(dirname(directory), { recursive: true });
        await this.writeLayer(images, runtimes, written, "linked");
        await files.rename(written, directory).catch(async (error: unknown) => {
            if (!FileSystemError.is(error, "FILE_EXISTS")) {
                throw error;
            }
            await files.remove(written, { recursive: true });
        });

        return directory;
    }

    /** Write what the images' runtimes run as packs of each build's files, the index last. */
    async #writePacked(
        images: ReadonlyMap<Digest, BuildImage>,
        runtimes: readonly Runtime[],
        directory: string,
    ): Promise<void> {
        // start from an empty directory of blobs
        const { files } = this.environment;
        const blobs = join(directory, LAYER_BLOBS);
        await files.remove(directory, { recursive: true, force: true });
        await files.mkdir(blobs, { recursive: true });

        // list the objects a pull of the images for the runtimes reads
        const roots = [...images.keys()].map((digest) => ({ kind: "image", digest }) as const);
        const objects = await BuildObject.runtime(this, roots, runtimes);
        const wanted = new Set(
            objects.filter((object) => object.kind === "file").map((object) => object.digest),
        );

        // link the builtins among the images, as a machine shipping them links their releases
        const linked = new Map<string, BuildImage>();
        for (const image of images.values()) {
            const { manifest } = await this.contents(image.root);
            if (manifest.role === "builtin") {
                linked.set(PackageManifest.package(manifest).name, image);
            }
        }

        // read the modules each image's root reaches on the runtimes loading modules by import
        const reached = new Set<string>();
        for (const image of images.values()) {
            for (const runtime of runtimes.filter((each) => LOADED_RUNTIMES.has(each))) {
                const map = await BuildImage.imports(image, runtime, LAYER_BASE, linked, (build) =>
                    this.get(build),
                );
                const root = await this.contents(image.root);
                const modules = await ImportMap.modules(
                    map,
                    LAYER_BASE,
                    runtime,
                    await root.entries(runtime),
                    (build) => this.contents(build),
                );
                for (const name of modules.keys()) {
                    reached.add(name);
                }
            }
        }

        // write each build's manifest at its digest, and list its wanted files that run
        const sizes: BuildLayerIndex["builds"] = {};
        const sources = new Map<Digest, { readonly reader: BuildReader; readonly path: string }>();
        const groups: Pick<BuildLayerFile, "digest" | "size">[][] = [];
        const listed = new Map<Digest, PackageFile[]>();
        for (const { digest } of objects.filter((object) => object.kind === "manifest")) {
            const reader = await this.contents(digest);
            const manifest = PackageManifest.serialize(reader.manifest);
            await files.writeFile(join(blobs, digest), manifest);
            sizes[digest] = manifest.byteLength;

            // group the build's files no earlier build listed
            const distributed = await reader.references();
            listed.set(digest, distributed);
            const unreached = await reader.unused(distributed, reached);
            const added = distributed
                .filter((file) => wanted.has(file.digest) && !unreached.has(file.digest))
                .filter((file) => !sources.has(file.digest));
            const group = [...new Map(added.map((file) => [file.digest, file])).values()];
            for (const file of group) {
                sources.set(file.digest, { reader, path: file.path });
            }
            groups.push(group);
        }

        // write each pack: its files' bytes end to end
        const packs = BuildLayer.pack(groups, PACK_BYTES);
        await files.mkdir(join(directory, LAYER_PACKS));
        for (const [number, pack] of packs.entries()) {
            const loaded: Uint8Array<ArrayBuffer>[] = [];
            for (const entry of pack) {
                const source = present(
                    sources.get(entry.digest),
                    `the packed file ${entry.digest}`,
                );
                loaded.push(await source.reader.load(source.path));
            }
            const bytes = new Uint8Array(await new Blob(loaded).arrayBuffer());
            await files.writeFile(join(directory, LAYER_PACKS, String(number)), bytes);
        }

        // write the index
        const packed = packs.map((pack) => pack.reduce((sum, entry) => sum + entry.size, 0));
        const placed = packs.flat();
        await BuildLayer.writeIndex(
            directory,
            images,
            sizes,
            listed,
            "pack",
            placed,
            packed,
            files,
        );
    }

    /** Write a build into the store's layered builds once, unless another writer did first. */
    async #writePooled(reader: BuildReader, build: string): Promise<void> {
        // leave a build the pool has
        const { files } = this.environment;
        if (await files.exists(join(build, MANIFEST_PATH))) {
            return;
        }

        // write it in this process's temporary directory, then move it in
        await files.mkdir(dirname(build), { recursive: true });
        const staging = await this.makeTemporaryDirectory("pooled-");
        try {
            // write every file, as linked layers of any runtimes share the pooled build
            await this.#writeDirectory(reader, join(staging, "build"), await reader.references());
            await files.rename(join(staging, "build"), build).catch(async (error: unknown) => {
                if (!(await files.exists(join(build, MANIFEST_PATH)))) {
                    throw error;
                }
            });
        } finally {
            await files.remove(staging, { recursive: true, force: true });
        }
    }

    /** Clone some files of a build into a new directory, its manifest last. */
    async #writeDirectory(
        reader: BuildReader,
        directory: string,
        selected: readonly PackageFile[],
    ): Promise<void> {
        // require a new directory
        const { files } = this.environment;
        await files.mkdir(directory);
        await this.place(
            selected.map((file) => ({
                file,
                destination: join(directory, PackagePath.parse(file.path)),
            })),
        );

        // make the build readable only after every file is written
        const manifest = PackageManifest.serialize(reader.manifest);
        await files.writeFile(join(directory, MANIFEST_PATH), manifest, { createNew: true });
    }

    /** Lay out the tree an image runs on a runtime, once per image, runtime and builtins. */
    tree(
        image: BuildImage,
        runtime: Runtime,
        builtins: ReadonlyMap<string, BuildImage>,
    ): Promise<BuildTree> {
        return this.scope.span("destack.build.tree", {}, () =>
            this.#tree(image, runtime, builtins),
        );
    }

    /** Open the tree realized for an image's inputs, else realize it and record it. */
    async #tree(
        image: BuildImage,
        runtime: Runtime,
        builtins: ReadonlyMap<string, BuildImage>,
    ): Promise<BuildTree> {
        // open the tree the same image, runtime and builtins realized before
        const { files, crypto, random } = this.environment;
        const inputs = {
            image,
            runtime,
            builtins: [...builtins].toSorted(([left], [right]) => (left < right ? -1 : 1)),
        };
        const record = join(this.realized, `${await Digest.json(inputs, crypto)}.json`);
        if (await files.exists(record)) {
            const directories = RealizedTree.parse(JSON.parse(await files.readTextFile(record)));

            return await BuildTree.read(directories, files);
        }

        // realize the tree, then record its directories by renaming a complete file into place
        const tree = await this.#realize(image, runtime, builtins);
        const written = `${record}.${Uuid.v4(random)}`;
        await files.mkdir(this.realized, { recursive: true });
        await files.writeTextFile(written, JSON.stringify(tree.directories));
        await files.rename(written, record);

        return tree;
    }

    /** Place the builds an image runs with. */
    async #realize(
        image: BuildImage,
        runtime: Runtime,
        builtins: ReadonlyMap<string, BuildImage>,
    ): Promise<BuildTree> {
        // read the closures of the image and the builtins it reaches
        const find = async (needed: Digest) => {
            const stored = await this.find(needed);
            if (stored === undefined) {
                throw BuildError.of(
                    "DEPENDENCY_INVALID",
                    `build '${needed}' of image root '${image.root}' is invalid: it is missing from the store`,
                );
            }

            return stored;
        };
        const map = await BuildImage.imports(image, runtime, LAYER_BASE, builtins, find);
        const mapped = ImportMap.builds(map, LAYER_BASE);
        const images = [image, ...[...builtins.values()].filter((each) => mapped.has(each.root))];
        const loads = await Promise.all(
            images.map((each) => BuildImage.loads(each, runtime, find, "export")),
        );
        const closure = new Set(loads.flatMap((each) => [...each]));

        // list each image's releases once
        const listed = await Promise.all(
            images.map((each) => BuildImage.releases(each, (digest) => this.get(digest))),
        );
        const releases = [...new Map(listed.flat().map((each) => [each.manifest, each])).values()];

        // resolve a name through its image, else to its builtin
        const resolve = (build: Digest, name: string) => {
            const listing = images.find((each) => BuildImage.lists(each, build));
            const pinned =
                listing === undefined ? undefined : BuildImage.resolve(listing, build, name);

            return pinned ?? builtins.get(name)?.root;
        };

        return await BuildTree.materialize(this, releases, resolve, this.trees, {
            kind: "run",
            runtime,
            closure,
        });
    }

    /** Place a tree's root once among the trees placing it at the same time. */
    placeRoot(root: string, place: () => Promise<void>): Promise<void> {
        // share the placement under way, forgetting it once it ends
        const placing =
            this.#placing.get(root) ?? place().finally(() => this.#placing.delete(root));
        this.#placing.set(root, placing);

        return placing;
    }

    /** Write files to paths, cloning stored blobs and the layer's files. */
    async place(placements: readonly FilePlacement[]): Promise<void> {
        // create each directory once
        const { files } = this.environment;
        const directories = new Set(placements.map(({ destination }) => dirname(destination)));
        for (const directory of [...directories].toSorted()) {
            await files.mkdir(directory, { recursive: true });
        }

        // find where each file lies, then clone it a batch at a time
        const located = await this.locate(placements.map(({ file }) => file.digest));
        for (let start = 0; start < placements.length; start += PLACE_BATCH) {
            await Promise.all(
                placements.slice(start, start + PLACE_BATCH).map(({ file, destination }) => {
                    const location = present(
                        located.get(file.digest),
                        `package file '${file.path}'`,
                    );

                    return location.kind === "layered"
                        ? this.placeLayered(file, destination, files)
                        : this.blobs.storage.copy(location, destination);
                }),
            );
        }
    }

    /** Forget deleted directories' roots, collect, then delete deleted builds' trees. */
    async #collectIdle(): Promise<void> {
        // forget each root of a directory that no longer exists
        for (const name of (await this.roots()).keys()) {
            const directory = name.startsWith(DIRECTORY_ROOT_PREFIX)
                ? name.slice(DIRECTORY_ROOT_PREFIX.length)
                : undefined;
            if (directory !== undefined && !(await this.environment.files.exists(directory))) {
                await this.forget(name);
            }
        }

        // delete what ended processes and blob writes left
        const { files } = this.environment;
        await this.#deleteEndedTemporaries();
        await this.blobs.storage.clean();

        // drop the unused cache entries and fit the size, then delete what nothing retains
        await this.cache.expire();
        await this.cache.evict(MAX_BYTES);
        await this.blobs.collect(0);

        // delete the trees of the builds whose manifests the store no longer has, whole or in part
        const entries = await files.readDirectory(this.trees);
        const trees = entries
            .filter((entry) => entry.kind === "directory")
            .map((entry) => ({
                name: entry.name,
                manifest: BuildTree.manifestOf(entry.name),
            }));
        const manifests = await this.#stored(trees.map((tree) => tree.manifest));
        for (const tree of trees) {
            if (tree.manifest === undefined || !manifests.has(tree.manifest)) {
                await files.remove(join(this.trees, tree.name), { recursive: true, force: true });
            }
        }

        // delete interrupted records, and the records of realized trees whose directories are gone
        for (const name of await LocalBuildStore.#entries(this.realized, files)) {
            const path = join(this.realized, name);
            const directories = name.endsWith(".json")
                ? RealizedTree.parse(JSON.parse(await files.readTextFile(path)))
                : {};
            const isWhole = await Promise.all(
                Object.values(directories).map((directory) => files.exists(directory)),
            );
            if (!name.endsWith(".json") || isWhole.includes(false)) {
                await files.remove(path, { force: true });
            }
        }

        // delete the layered builds of other layer formats
        const layered = join(this.directory, LAYERED_DIRECTORY);
        for (const entry of await LocalBuildStore.#entries(layered, files)) {
            if (entry !== String(LAYER_FORMAT)) {
                await files.remove(join(layered, entry), { recursive: true, force: true });
            }
        }

        // delete the layered builds the store no longer has, and what interrupted writes left
        const pooled = (await LocalBuildStore.#entries(this.#pool, files)).map((name) => ({
            name,
            digest: Digest.safeParse(name).data,
        }));
        const kept = await this.#stored(pooled.map((entry) => entry.digest));
        for (const entry of pooled) {
            if (entry.digest === undefined || !kept.has(entry.digest)) {
                await files.remove(join(this.#pool, entry.name), { recursive: true, force: true });
            }
        }

        // delete the layers linking a build the store no longer has
        const layers = join(this.directory, LAYERS_DIRECTORY);
        for (const entry of await LocalBuildStore.#entries(layers, files)) {
            const text = await files
                .readTextFile(join(layers, entry, LAYER_INDEX))
                .catch(FileSystemError.absent);
            const written =
                text === undefined ? undefined : JsonValue.object(JsonValue.parse(text));
            const index =
                written?.["formatVersion"] === LAYER_FORMAT
                    ? BuildLayerIndex.parse(written)
                    : undefined;
            const builds = Object.keys(index?.builds ?? {}).map((each) => Digest.parse(each));
            const isComplete =
                index !== undefined && (await this.has(builds)).every((isStored) => isStored);
            if (!isComplete) {
                await files.remove(join(layers, entry), { recursive: true, force: true });
            }
        }
    }

    /** List a directory's entries by name, none for a directory that does not exist. */
    static async #entries(directory: string, files: FileSystem): Promise<string[]> {
        const entries = await files.readDirectory(directory).catch(FileSystemError.absent);

        return (entries ?? []).map((entry) => entry.name);
    }

    /** Find which of some blobs the store has, at once, leaving out absent digests. */
    async #stored(digests: readonly (Digest | undefined)[]): Promise<ReadonlySet<Digest>> {
        const given = digests.filter((digest) => digest !== undefined);
        const isStored = await this.has(given);

        return new Set(given.filter((_digest, index) => isStored[index] === true));
    }

    /** Collect an idle store a day after its last collection, recording when it did. */
    async #collectIdleIfDue(path: string): Promise<void> {
        // read when the store last collected, never for a new store
        const { files, clock } = this.environment;
        const now = clock.now();
        const collected = (await files.exists(path))
            ? schema.int().parse(Number(await files.readTextFile(path)))
            : undefined;

        // collect once the day since passed, and record it
        if (collected === undefined || now - collected >= COLLECT_MILLISECONDS) {
            await this.#collectIdle();
            await files.writeTextFile(path, String(now));
        }
    }

    /** Open the store in a directory, collecting it daily when no other process has it open. */
    static async open(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalBuildStore> {
        // lock the directory while one process opens and collects the store
        const { files } = environment;
        await files.mkdir(directory, { recursive: true });
        const held = await files.lock(join(directory, OPEN_LOCK), "exclusive");
        try {
            // learn whether another process has the store open, as openers wait here
            const location = await files.realPath(directory);
            const idle = await files.tryLock(join(location, STORE_LOCK), "exclusive");
            await idle?.close();

            // open the store sharing its image, then collect it while no other process builds into it
            const disposables = new AsyncDisposableStack();
            try {
                const shared = await files.lock(join(location, STORE_LOCK), "shared");
                disposables.defer(() => shared.close());
                const store = await LocalBuildStore.#openDirectory(
                    location,
                    environment,
                    disposables,
                );
                if (idle !== undefined) {
                    await store.#collectIdleIfDue(join(location, COLLECTED_FILE));
                }

                return store;
            } catch (error) {
                // release the image and whatever opened once opening fails
                await disposables.disposeAsync();
                throw error;
            }
        } finally {
            await held.close();
        }
    }

    /** Open the person's store for one use, in their cache directory or the environment's. */
    static async user(
        use: LocalStoreUse,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalBuildStore> {
        // take the cache directory the environment names, as NX_CACHE_DIRECTORY does, else the person's
        const { DESTACK_CACHE_DIRECTORY: named } = environment.process.variables;
        if (named === "") {
            throw BuildError.of(
                "CACHE_DIRECTORY_INVALID",
                "DESTACK_CACHE_DIRECTORY is invalid: it is empty",
            );
        }
        const cache = named ?? join(await environment.files.cacheDirectory(), CACHE_DIRECTORY);

        return await LocalBuildStore.open(join(cache, use), environment);
    }

    /** Open a store of one run's builds, removed on disposal. */
    static async temporary(
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalBuildStore> {
        // open a store in a new directory, both released on disposal
        const { files } = environment;
        const temporary = await files.makeTemporaryDirectory("destack-store-");
        const directory = await files.realPath(temporary);
        const disposables = new AsyncDisposableStack();
        disposables.defer(() => files.remove(directory, { recursive: true }));

        return await LocalBuildStore.#openDirectory(directory, environment, disposables);
    }

    /** Close the database the store opened, removing a temporary store. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposables.disposeAsync();
    }
}
