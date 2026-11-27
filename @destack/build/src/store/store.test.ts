import { Tarball, TarballEntry } from "@destack/package/archive";
import { testSimulation } from "../test/world.ts";
import type { Crypto, Environment, FileSystem } from "@destack/environment";
import { expect, refusal, test } from "@destack/test";
import { join } from "node:path";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore } from "@destack/db/database";
import { connectBunSqlite } from "@destack/db/bun";
import { graph, PackageId } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import {
    BuildReader,
    BuildWriter,
    type PackageOutput,
    BuildImage,
    PackageManifest,
    BuildListCache,
} from "@destack/package/manifest";
import { CallError } from "@destack/object";
import { Digest, found, present } from "@destack/schema";
import { BuildObject, type PackageAccess, RemoteBuildStore, BuildStore } from "./index.ts";
import { buildManifest } from "./table.ts";

/** The mount the tests serve stores below. */
const MOUNT = new URL("https://packages.test/packages/");
/** The source files of the fixture build. */
const SOURCES = {
    "src/answer.ts": "export const answer = 42;\n",
    "src/index.ts": 'export * from "./answer.ts";\n',
};

/** How long a store leases what it stores for a root to retain: half an hour. */
const LEASE_MILLISECONDS = 30 * 60_000;

/** A week, as long as a use leases a build. */
const WEEK_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How long a cache entry retains what it references past its last use: a week. */
const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** A day, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60_000;

/** A minute, in milliseconds. */
const MINUTE_MILLISECONDS = 60_000;

/** The builds the many-builds collection test stores, half of them retained. */
const STRESS_BUILDS = 20;

/** A host granting every pull and every push. */
const PUSHING: PackageAccess = { pull: async () => {}, push: async () => {} };

/** A host granting every pull at a read-only endpoint. */
const PULLING: PackageAccess = { pull: async () => {} };

/** Create a temporary directory in a world. */
async function temporary(files: FileSystem): Promise<string> {
    return await files.makeTemporaryDirectory("store-");
}

/** The package the stored builds build, unless one names another. */
const ANSWER: PackageManifest["package"] = {
    id: PackageId.parse("package-00000000-0000-7000-8000-00000000a001"),
    name: "@example/answer",
    version: "2026.9.0",
};

/** The npm release the tiny unit builds. */
const TINY: PackageManifest["package"] = {
    name: "tiny",
    version: "1.0.0",
    integrity: "sha512-tiny",
};

/** Write a build of some source files in memory, as a builder writes one. */
async function written(
    sources: Readonly<Record<string, string>>,
    crypto: Crypto,
    outputs: Readonly<Record<string, PackageOutput>> = {},
    built: PackageManifest["package"] = ANSWER,
): Promise<BuildReader> {
    // write each source file, then the lists and the manifest
    const bytes = new Map<string, Uint8Array<ArrayBuffer>>();
    const writer = new BuildWriter(
        { write: async (path, contents) => void bytes.set(path, contents) },
        crypto,
    );
    for (const [path, text] of Object.entries(sources)) {
        await writer.write(path, new TextEncoder().encode(text));
    }
    const manifest = await writer.finish({
        destack: "2026.11.0",
        role: "app",
        sideEffects: true,
        package: built,
        outputs,
    });

    return new BuildReader(
        manifest,
        async (path) => new Blob([found(bytes, path)]).stream(),
        crypto,
        new BuildListCache(),
    );
}

/** Describe a build's Bun output: its exports' imports and its workloads. */
function bunOutputs(
    imports: Readonly<Record<string, readonly string[]>>,
    entries: readonly string[] = [],
): Readonly<Record<string, PackageOutput>> {
    // export each subpath at a file of its own, and run each entry as a workload
    const exports = Object.fromEntries(
        Object.entries(imports).map(([subpath, specifiers]) => [
            subpath,
            { path: `bun/${subpath.slice(2) || "index"}.js`, imports: [...specifiers] },
        ]),
    );
    const workloads = Object.fromEntries(
        entries.map((entrypoint) => [
            entrypoint.slice(2),
            {
                entrypoint,
                packages: [],
                resources: [],
                secrets: [],
                compute: {},
                sandbox: {},
            },
        ]),
    );

    return {
        bun: {
            runtime: "bun",
            emit: true,
            workloads,
            views: {},
            directory: "bun",
            exports,
            peers: {},
            inlined: {},
        },
    };
}

/** Make an image of a root build linking each package's build it runs with, which link none. */
function imageOf(root: Digest, builds: Readonly<Record<string, Digest>>): BuildImage {
    const leaves = Object.values(builds).map((build) => [build, {}] as const);

    return BuildImage.of(root, { ...Object.fromEntries(leaves), [root]: builds });
}

/** Read a stream's bytes. */
async function read(stream: ReadableStream<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** A store over blobs and a database in a directory, closing its database on disposal. */
interface OpenedStore extends AsyncDisposable {
    /** The store. */
    readonly store: BuildStore;
}

/** Open a store over the blobs and the database in a directory, created once. */
async function openStore(
    directory: string,
    environment: Environment<"files" | "process">,
): Promise<OpenedStore> {
    const storage = await LocalBlobStorage.open(join(directory, "files"), environment, {
        synchronous: "off",
    });
    const database = await connectBunSqlite(join(directory, "store.db"), BuildStore.database, {
        environment,
    });
    await database.migrate(BuildStore.database.tables);

    return {
        store: new BuildStore({
            blobs: new BlobStore({ storage, database, environment }),
            environment,
        }),
        [Symbol.asyncDispose]: () => database.close(),
    };
}

/** List which of some blobs the store has, in order. */
async function listing(store: BuildStore, digests: readonly Digest[]): Promise<Digest[]> {
    const stored = await store.has(digests);

    return digests.filter((_digest, index) => stored[index] === true).toSorted();
}

/** Count the bytes of some stored builds' blobs, each blob once. */
async function blobsSize(
    store: BuildStore,
    manifests: readonly (Digest | undefined)[],
): Promise<number> {
    const digests = await blobsOf(
        store,
        manifests.map((digest) => present(digest, "a build")),
    );
    const sizes = await Promise.all(
        digests.map(async (digest) => (await store.blobs.bytes(Digest.parse(digest))).byteLength),
    );

    return sizes.reduce((total, size) => total + size, 0);
}

/** List the blobs of some stored builds: their manifests and every file they list. */
async function blobsOf(store: BuildStore, manifests: readonly Digest[]): Promise<Digest[]> {
    const files = await Promise.all(
        manifests.map(async (digest) => (await store.contents(digest)).references()),
    );

    return [...new Set([...manifests, ...files.flat().map((file) => file.digest)])].toSorted();
}

test("store a build's files once by digest, read it after reopening, and refuse stored corruption", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    const build = await written(SOURCES, environment.crypto);
    const list = await build.references();
    const answer = present(
        list.find((file) => file.path === "src/answer.ts"),
        "the answer module",
    );

    // store the build twice, which writes every file once under its digest beside its manifest
    let digest: Digest;
    {
        await using opened = await openStore(directory, environment);
        const { store } = opened;
        digest = await store.put(build);
        const blobs = [...new Set([...list.map((file) => file.digest), digest])].toSorted();
        expect([await store.put(build), await listing(store, blobs)]).toEqual([digest, blobs]);
    }

    // read the same build after reopening the store
    await using opened = await openStore(directory, environment);
    const { store } = opened;
    expect([await store.get(digest), await store.file(answer)]).toEqual([
        build.manifest,
        new TextEncoder().encode(SOURCES["src/answer.ts"]),
    ]);

    // fail reads of stored corruption, and take the stored build by its manifest without rewriting it
    const packs = await files.readDirectory(join(directory, "files"));
    for (const pack of packs.filter((entry) => entry.kind === "file")) {
        await files.writeFile(
            join(directory, "files", pack.name),
            new TextEncoder().encode("broken"),
        );
    }
    const corrupt = [
        "BAD_REQUEST",
        "FILE_INVALID",
        "file 'src/answer.ts' is invalid: its size does not match",
    ];
    expect([
        await refusal(store.file(answer)),
        await store.put(build),
        await refusal(store.file(answer)),
    ]).toEqual([corrupt, digest, corrupt]);
});

test("collect the builds no root or lease retains, the blobs only they reference, and unused cache entries", async () => {
    // store a rooted build and a dropped one sharing the index module, each cached by a key
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const live = await store.put(await written(SOURCES, environment.crypto));
    const dropped = await store.put(
        await written(
            { ...SOURCES, "src/answer.ts": "export const answer = 43;\n" },
            environment.crypto,
        ),
    );
    const keys = { live: Digest.parse("a".repeat(64)), dropped: Digest.parse("b".repeat(64)) };
    await store.cache.set(keys.live, { kind: "build", manifest: live });
    await store.cache.set(keys.dropped, { kind: "build", manifest: dropped });
    await store.retain("space-a", new Set([live]));
    const both = await blobsOf(store, [live, dropped]);

    // collect while the stores' leases and the cache entries retain everything
    const leased = await collect(store);
    const before = await listing(store, both);

    // collect once the leases end, then use the live entry a day on, and collect once the other went unused for a week
    const start = simulation.clock.now();
    await simulation.jump(simulation.clock.now() + LEASE_MILLISECONDS);
    const cached = await collect(store);
    await simulation.jump(simulation.clock.now() + DAY_MILLISECONDS - LEASE_MILLISECONDS);
    await store.cache.get(keys.live);
    await simulation.jump(simulation.clock.now() + CACHE_MILLISECONDS - MINUTE_MILLISECONDS);
    const unused = await collect(store);

    // keep exactly the live build's blobs, answering when each collection could delete more
    expect({
        leased,
        before,
        cached,
        unused,
        after: await listing(store, both),
        entries: [await store.cache.get(keys.live), await store.cache.get(keys.dropped)],
    }).toEqual({
        leased: { manifests: [], next: start + LEASE_MILLISECONDS },
        before: both,
        cached: { manifests: [], next: start + CACHE_MILLISECONDS },
        unused: { manifests: [dropped], next: start + DAY_MILLISECONDS + CACHE_MILLISECONDS },
        after: await blobsOf(store, [live]),
        entries: [{ kind: "build", manifest: live }, undefined],
    });
});

test("attach a file to a build, retain it with the build, and collect it with the build", async () => {
    // store two builds, each with a separate file attached, rooting the first
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const attached = async (sources: Readonly<Record<string, string>>, index: string) => {
        const manifest = await store.put(await written(sources, environment.crypto));
        const bytes = new TextEncoder().encode(index);
        const file = await PackageFile.describe(
            "code.json",
            "application/json",
            bytes,
            environment.crypto,
        );
        await store.write(BuildObject.files([{ digest: file.digest, bytes }]));
        await store.attach(manifest, [file]);

        return { manifest, file };
    };
    const live = await attached(SOURCES, "[]");
    const dropped = await attached(
        { ...SOURCES, "src/answer.ts": "export const answer = 43;\n" },
        "[ ]",
    );
    await store.retain("space-a", new Set([live.manifest]));
    const builds = await blobsOf(store, [live.manifest, dropped.manifest]);
    const all = [...new Set([...builds, live.file.digest, dropped.file.digest])];

    // refuse a file the store lacks, and read each attached index back
    const missing = { ...live.file, digest: Digest.parse("c".repeat(64)) };
    await expect(store.attach(live.manifest, [missing])).rejects.toThrow(
        `build '${live.manifest}' is invalid: it lists 'code.json', which was not uploaded`,
    );
    expect([
        await store.attachment(live.manifest, "code.json"),
        await store.attachment(dropped.manifest, "code.json"),
        await store.attachment(live.manifest, "other.json"),
    ]).toEqual([live.file, dropped.file, undefined]);

    // collect the dropped build once the leases end, its index and its attachment with it
    await simulation.jump(simulation.clock.now() + LEASE_MILLISECONDS);
    await collect(store);
    expect({
        blobs: await listing(store, all),
        attachments: [
            await store.attachment(live.manifest, "code.json"),
            await store.attachment(dropped.manifest, "code.json"),
        ],
    }).toEqual({
        blobs: [
            ...new Set([...(await blobsOf(store, [live.manifest])), live.file.digest]),
        ].toSorted(),
        attachments: [live.file, undefined],
    });
});

test("read a build, its image, files and attachments through a source, reporting only its own builds as contained", async () => {
    // store a build with an image and an attached index in a source, and open an empty store over it
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using kept = await openStore(await temporary(environment.files), environment);
    await using empty = await openStore(await temporary(environment.files), environment);
    const source = kept.store;
    const manifest = await source.put(await written(SOURCES, environment.crypto));
    const image = await source.putImage(BuildImage.of(manifest));
    const bytes = new TextEncoder().encode("[]");
    const index = await PackageFile.describe(
        "code.json",
        "application/json",
        bytes,
        environment.crypto,
    );
    await source.write(BuildObject.files([{ digest: index.digest, bytes }]));
    await source.attach(manifest, [index]);
    const store = new BuildStore({ blobs: empty.store.blobs, environment, sources: [source] });

    // read the build, its root through the image, a file, the index and which store keeps it
    const reader = await store.contents(manifest);
    const root = await store.root(image);
    const file = await reader.file("src/answer.ts");
    expect({
        manifest: reader.manifest,
        root: root.manifest,
        file: new TextDecoder().decode(await store.file(file)),
        attachment: await store.attachment(manifest, "code.json"),
        isKeeper: (await store.storeOf({ kind: "manifest", digest: manifest })) === source,
        contains: [await store.contains(manifest), await source.contains(manifest)],
    }).toEqual({
        manifest: (await source.contents(manifest)).manifest,
        root: (await source.contents(manifest)).manifest,
        file: SOURCES["src/answer.ts"],
        attachment: index,
        isKeeper: true,
        contains: [false, true],
    });
});

test("pull a build's graph from its forge on the first read, once, and refuse a build without one", async () => {
    // store a build with its graph at a forge, and the build alone here, pulling attachments from the forge
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using forge = await openStore(await temporary(environment.files), environment);
    await using local = await openStore(await temporary(environment.files), environment);
    const build = await written(SOURCES, environment.crypto);
    const manifest = await forge.store.put(build);
    await local.store.put(build);
    const module = graph.Module.of({
        path: "src/answer.ts",
        digest: Digest.parse("a".repeat(64)),
        imports: [],
        exports: [],
        declarations: [],
        symbols: [],
        edges: [],
    });
    await forge.store.attachGraph(manifest, [module]);
    const pulls: Digest[] = [];
    const store = new BuildStore({
        blobs: local.store.blobs,
        environment,
        pull: async (_packageId, digest) => {
            pulls.push(digest);
            const files = await forge.store.attachments(digest);
            if (files.length === 0) {
                return;
            }
            const objects = files.map((file) => ({ kind: "file", digest: file.digest }) as const);
            await local.store.write(forge.store.read(objects));
            await local.store.attach(digest, files);
        },
    });

    // read the graph twice, pulling it once, and refuse a build its forge attached none to
    const other = await store.put(
        await written(
            { ...SOURCES, "src/answer.ts": "export const answer = 43;\n" },
            environment.crypto,
        ),
    );
    expect({
        isIndexed: await store.isIndexed(manifest),
        first: await store.graph(manifest),
        second: await store.graph(manifest),
        pulls,
        refused: await refusal(store.graph(other)),
    }).toEqual({
        isIndexed: false,
        first: [module],
        second: [module],
        pulls: [manifest, other],
        refused: ["NOT_FOUND", "GRAPH_NOT_FOUND", `graph of build '${other}' not found`],
    });
});

test("open a stored build, find its manifest, and refuse a collected build or find it absent", async () => {
    // store a build and open it twice, each time with a reader of its own
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const digest = await store.put(await written(SOURCES, environment.crypto));
    const [first, second] = [await store.contents(digest), await store.contents(digest)];
    const manifest = await store.find(digest);

    // collect it once its lease ends, then open it again
    await simulation.jump(simulation.clock.now() + LEASE_MILLISECONDS);
    await collect(store);
    const reopened = await store.contents(digest).then(
        () => "opened",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
    const collected = await store.find(digest);

    // read the same manifest through each reader while the build is stored, and none once collected
    expect({
        isShared: first === second,
        manifests: [first.manifest, second.manifest],
        reopened,
        collected,
    }).toEqual({
        isShared: false,
        manifests: [manifest, manifest],
        reopened: `package manifest '${digest}' not found`,
        collected: undefined,
    });
});

test("retain a leased build until its lease ends, then collect it with every blob", async () => {
    // store a build and lease it for a day
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const stored = simulation.clock.now();
    const digest = await store.put(await written(SOURCES, environment.crypto));
    const build = await blobsOf(store, [digest]);
    const until = stored + DAY_MILLISECONDS;
    await store.lease(digest, new Date(until));

    // collect within the lease, and once it ends
    const within = await collect(store);
    const kept = await listing(store, build);
    await simulation.jump(simulation.clock.now() + until - simulation.clock.now());
    const ended = await collect(store);
    expect({ within, kept, ended, after: await listing(store, build) }).toEqual({
        within: { manifests: [], next: stored + LEASE_MILLISECONDS },
        kept: build,
        ended: { manifests: [digest], next: undefined },
        after: [],
    });
});

test("store a build once for callers putting it at once, refusing only the caller whose signal aborts", async () => {
    // put one build three times at once, one caller aborting while the storing goes on
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const build = await written(SOURCES, environment.crypto);
    const aborting = new AbortController();
    const first = store.put(build);
    const left = store.put(build, aborting.signal).then(
        () => "stored",
        (error: unknown) => (error instanceof Error ? error.message : error),
    );
    const last = store.put(build);
    aborting.abort(new Error("caller left"));

    // answer the others with the one digest and its stored files, the aborted caller with its reason
    const digest = await first;
    const blobs = await blobsOf(store, [digest]);
    expect({ last: await last, left: await left, stored: await listing(store, blobs) }).toEqual({
        last: digest,
        left: "caller left",
        stored: blobs,
    });
});

test("rescue a blob a collection listed as garbage once a writer leases it, and store it again", async () => {
    // collect an unrooted build down to garbage rows, its blobs not deleted yet
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const build = await written(SOURCES, environment.crypto);
    const digest = await store.put(build);
    await simulation.jump(simulation.clock.now() + LEASE_MILLISECONDS);
    await collect(store);

    // store the same build again after the collection, which writes every blob back
    const again = await store.put(build);
    const blobs = await blobsOf(store, [again]);
    await store.retain("space-a", new Set([again]));
    await simulation.jump(simulation.clock.now() + LEASE_MILLISECONDS);
    await collect(store);
    expect({
        again,
        isStored: await store.contains(digest),
        blobs: await listing(store, blobs),
        manifest: await store.get(digest),
    }).toEqual({
        again: digest,
        isStored: true,
        blobs,
        manifest: build.manifest,
    });
});

test("collect many builds sharing a module, keeping exactly the rooted ones and the files they list", async () => {
    // store builds sharing the index module, each with a separate answer, rooting the first half
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const digests = [];
    for (let index = 0; index < STRESS_BUILDS; index++) {
        const answer = `export const answer = ${String(index)};\n`;
        digests.push(
            await store.put(
                await written({ ...SOURCES, "src/answer.ts": answer }, environment.crypto),
            ),
        );
    }
    const live = digests.slice(0, STRESS_BUILDS / 2);
    const all = await blobsOf(store, digests);
    await store.retain("space-a", new Set(live));

    // collect once the leases end, keeping exactly the live manifests and the files they list
    await simulation.jump(simulation.clock.now() + LEASE_MILLISECONDS);
    const collected = await collect(store);
    expect({
        manifests: [...collected.manifests].toSorted(),
        blobs: await listing(store, all),
    }).toEqual({
        manifests: digests.slice(STRESS_BUILDS / 2).toSorted(),
        blobs: await blobsOf(store, live),
    });
});

test("serve a stored build over HTTP after the host authorizes each read", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using opened = await openStore(join(directory, "store"), environment);
    const { store } = opened;
    const build = await written(SOURCES, environment.crypto);
    const digest = await store.put(build);
    const answer = present(
        (await build.references()).find((file) => file.path === "src/answer.ts"),
        "the answer module",
    );
    const bytes = new TextEncoder().encode(SOURCES["src/answer.ts"]);

    // grant bearers of the allowed token reads of the stored build
    const access = {
        async pull(request: Request, selected: string) {
            if (request.headers.get("authorization") !== "Bearer allowed" || selected !== digest) {
                throw CallError.http("UNAUTHORIZED", { message: "package access denied" });
            }
        },
    };
    const url = `https://packages.test/packages/${digest}/`;
    const fetchPackage = async (input: string | URL, options?: RequestInit) => {
        const request = new Request(String(input), options);
        request.headers.set("authorization", "Bearer allowed");

        return store.fetch(request, MOUNT, access);
    };

    // open the build as a remote package, refusing another manifest digest
    const reader = await BuildReader.open(
        { manifest: digest, url },
        fetchPackage,
        environment.crypto,
        new BuildListCache(),
    );
    expect([reader.manifest, await reader.files()]).toEqual([build.manifest, await build.files()]);
    expect(
        await refusal(
            BuildReader.open(
                { manifest: Digest.parse("0".repeat(64)), url },
                fetchPackage,
                environment.crypto,
                new BuildListCache(),
            ),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "FILE_INVALID",
        "file 'manifest.json' is invalid: its digest does not match",
    ]);

    // authorize before serving partial bodies or answering cache conditions
    const path = `${url}files/src/answer.ts`;
    const full = await fetchPackage(path);
    const partial = await fetchPackage(path, { headers: { range: "bytes=7-11" } });
    const cached = await fetchPackage(path, { headers: { "if-none-match": `"${answer.digest}"` } });
    const denied = await store.fetch(
        new Request(path, { headers: { "if-none-match": `"${answer.digest}"` } }),
        MOUNT,
        access,
    );
    const head = await fetchPackage(path, { method: "HEAD" });
    const missing = await fetchPackage(`${url}files/missing.ts`);
    expect({
        full: await read(present(full.body, "the full body")),
        partial: [partial.status, partial.headers.get("content-range"), await partial.text()],
        cached: cached.status,
        denied: [denied.status, await denied.json()],
        missing: missing.status,
        head: [head.status, head.headers.get("content-length"), await head.text()],
    }).toEqual({
        full: bytes,
        partial: [206, `bytes 7-11/${bytes.length}`, SOURCES["src/answer.ts"].slice(7, 12)],
        cached: 304,
        denied: [
            401,
            {
                defined: true,
                code: "UNAUTHORIZED",
                status: 401,
                message: "package access denied",
                reason: "UNAUTHORIZED",
                domain: "http",
            },
        ],
        missing: 404,
        head: [200, String(bytes.length), ""],
    });

    // release streamed reads when clients disconnect
    await present((await fetchPackage(path)).body, "the file body").cancel();
});

test("open a served image's root build", async () => {
    // serve a stored image of the fixture build to every reader
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const build = await written(SOURCES, environment.crypto);
    const image = await store.putImage(imageOf(await store.put(build), {}));
    const serving = (request: Request) => store.fetch(request, MOUNT, PUSHING);

    // open the image's root build below the mount
    const reader = await RemoteBuildStore.open(MOUNT.href, serving, image, environment.crypto);
    expect([reader.manifest, await reader.files()]).toEqual([build.manifest, await build.files()]);
});

test("list and read a served build's attachments below the path its reads take", async () => {
    // serve a stored build with a file attached to it
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const manifest = await store.put(await written(SOURCES, environment.crypto));
    const bytes = new TextEncoder().encode("[]");
    const file = await PackageFile.describe(
        "code.json",
        "application/json",
        bytes,
        environment.crypto,
    );
    await store.write(BuildObject.files([{ digest: file.digest, bytes }]));
    await store.attach(manifest, [file]);

    // list and read them through a remote store rooted at the build, as a pull of the build does
    const serving = (request: Request) => store.fetch(request, MOUNT, PUSHING);
    const root = BuildObject.path({ kind: "manifest", digest: manifest });
    const remote = new RemoteBuildStore(`${MOUNT.href}${root}/`, serving);
    const entries = await Array.fromAsync(remote.read([{ kind: "file", digest: file.digest }]));
    expect([await remote.attachments(), entries.map((entry) => entry.path)]).toEqual([
        [file],
        [BuildObject.path({ kind: "file", digest: file.digest })],
    ]);
});

test("refuse an image whose bytes do not match its digest", async () => {
    // serve a stored image, answering a read of another image with its bytes, as a mixed up server does
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const image = await store.putImage(
        imageOf(await store.put(await written(SOURCES, environment.crypto)), {}),
    );
    const other = Digest.parse("0".repeat(64));
    const mixing = async (request: Request) => {
        const body = (await request.text()).replaceAll(other, image);
        const url = request.url.replace(other, image);

        return await store.fetch(
            new Request(url, { method: request.method, headers: request.headers, body }),
            MOUNT,
            PUSHING,
        );
    };

    // refuse to open the image read under the other digest
    expect(
        await refusal(RemoteBuildStore.open(MOUNT.href, mixing, other, environment.crypto)),
    ).toEqual([
        "BAD_REQUEST",
        "MANIFEST_INVALID",
        `image '${other}' is invalid: its bytes do not match its digest`,
    ]);
});

test("pull a served build into another store in one read per round, reading nothing once stored", async () => {
    // serve a stored build, recording each path read
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using served = await openStore(join(directory, "served"), environment);
    await using copied = await openStore(join(directory, "copied"), environment);
    const source = served.store;
    const target = copied.store;
    const build = await written(SOURCES, environment.crypto);
    const digest = await source.put(build);
    const root = { kind: "manifest", digest } as const;
    const prefix = `/packages/${BuildObject.path(root)}/`;
    const paths: string[] = [];
    const client = new RemoteBuildStore(`https://packages.test${prefix}`, async (request) => {
        paths.push(new URL(request.url).pathname.slice(prefix.length));

        return source.fetch(request, MOUNT, { pull: async () => {} });
    });

    // pull the build, then pull it again, which reads nothing once the store has it
    const before = await target.contains(digest);
    await target.pull(client, root, "all");
    const first = paths.splice(0);
    await target.pull(client, root, "all");
    expect({
        before,
        after: await target.contains(digest),
        manifest: (await target.contents(digest)).manifest,
        first,
        second: paths,
    }).toEqual({
        before: false,
        after: true,
        manifest: build.manifest,
        first: ["read", "read", "read"],
        second: [],
    });
});

test("push a stored image to a store, writing only the objects it lacks", async () => {
    // store a build, and serve another store that accepts uploads
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using live = await openStore(join(directory, "live"), environment);
    await using received = await openStore(join(directory, "received"), environment);
    const source = live.store;
    const target = received.store;
    const build = await written(SOURCES, environment.crypto);
    const digest = await source.put(build);
    const uploads: string[] = [];
    const accepting = async (request: Request) => {
        uploads.push(`${request.method} ${new URL(request.url).pathname}`);

        return target.fetch(request, MOUNT, PUSHING);
    };

    // push the build's image, then push it again, which asks what the store has and writes nothing
    const image = await source.putImage(BuildImage.of(digest));
    const client = new RemoteBuildStore("https://packages.test/packages/", accepting);
    await client.push(source, [image]);
    const first = uploads.splice(0);
    await client.push(source, [image]);
    const list = await build.references();
    expect({
        manifest: (await target.contents(digest)).manifest,
        stored: await target.has(list.map((file) => file.digest)),
        first,
        second: uploads,
    }).toEqual({
        manifest: build.manifest,
        stored: list.map(() => true),
        first: ["POST /packages/has", "POST /packages/write"],
        second: ["POST /packages/has"],
    });
});

test("refuse writes of a manifest lacking files, a file of other bytes, one past the limit, and where nobody uploads", async () => {
    // store a build, and serve an empty store
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using live = await openStore(join(directory, "live"), environment);
    await using empty = await openStore(join(directory, "empty"), environment);
    const source = live.store;
    const digest = await source.put(await written(SOURCES, environment.crypto));
    const receiving = empty.store;
    const manifest = await source.file(await source.head(digest));

    // send a file of other bytes, and one a byte past the limit, each in a tarball
    const chunk = new Uint8Array(1024 * 1024);
    const oversized: TarballEntry = {
        path: BuildObject.path({ kind: "file", digest }),
        size: 64 * chunk.byteLength + 1,
        body: (async function* () {
            for (let index = 0; index < 64; index++) {
                yield chunk;
            }
            yield new Uint8Array(1);
        })(),
    };

    // refuse each write with its status and error
    const write = (entry: TarballEntry, access: PackageAccess) =>
        upload(
            receiving,
            "POST",
            "write",
            Tarball.stream(
                (async function* () {
                    yield entry;
                })(),
            ),
            access,
        );
    expect([
        await write(
            TarballEntry.of(BuildObject.path({ kind: "manifest", digest }), manifest),
            PUSHING,
        ),
        await write(
            TarballEntry.of(BuildObject.path({ kind: "file", digest }), new Uint8Array([1])),
            PUSHING,
        ),
        await write(oversized, PUSHING),
        await upload(receiving, "PUT", "write/extra", new Uint8Array([1]), PUSHING),
        await write(
            TarballEntry.of(BuildObject.path({ kind: "manifest", digest }), manifest),
            PULLING,
        ),
    ]).toEqual([
        [
            400,
            {
                defined: false,
                code: "BAD_REQUEST",
                status: 400,
                message: `build '${digest}' is invalid: it lists 'manifest/files.json', which was not uploaded`,
                reason: "MANIFEST_BLOB_INVALID",
                domain: "@destack/build",
            },
        ],
        [
            400,
            {
                defined: false,
                code: "BAD_REQUEST",
                status: 400,
                message: `object is invalid: it does not match its digest '${digest}'`,
                reason: "DIGEST_INVALID",
                domain: "@destack/build",
            },
        ],
        [
            400,
            {
                defined: false,
                code: "BAD_REQUEST",
                status: 400,
                message: `object '${digest}' is invalid: it exceeds ${64 * chunk.byteLength} bytes`,
                reason: "OBJECT_INVALID",
                domain: "@destack/build",
            },
        ],
        [405, null],
        [405, null],
    ]);
});

/** Upload a body to a store path as a host decides, returning the status and error. */
async function upload(
    store: BuildStore,
    method: "POST" | "PUT",
    path: string,
    body: BodyInit,
    access: PackageAccess,
): Promise<[number, unknown]> {
    const response = await store.fetch(
        new Request(`https://packages.test/packages/${path}`, {
            method,
            body,
            headers: { "content-type": "application/octet-stream" },
        }),
        MOUNT,
        access,
    );
    const text = await response.text();

    return [response.status, text === "" ? null : JSON.parse(text)];
}

test("retain the union of every root's builds, and collect a forgotten root's builds", async () => {
    // store two builds, each retained by a separate root, once their leases ended
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const first = await store.put(await written(SOURCES, environment.crypto));
    const second = await store.put(
        await written({ ...SOURCES, "src/extra.ts": "export {};\n" }, environment.crypto),
    );
    await simulation.jump(simulation.clock.now() + LEASE_MILLISECONDS);
    await store.retain("space-a", new Set([first]));
    await store.retain("space-b", new Set([second]));
    await collect(store);
    const both = await recorded(store);
    const all = await blobsOf(store, [first, second]);

    // forget one root and collect its build, leaving the other's retained
    await store.forget("space-a");
    await collect(store);
    expect({
        both: [...both].toSorted(),
        roots: [...(await store.roots()).keys()],
        isFirstStored: await store.contains(first),
        blobs: await listing(store, all),
    }).toEqual({
        both: [first, second].toSorted(),
        roots: ["space-b"],
        isFirstStored: false,
        blobs: await blobsOf(store, [second]),
    });
});

test("evict the least recently used builds until the store fits its size, keeping a rooted and a recently used one", async () => {
    // store three builds a minute apart, rooting the oldest and using the newest
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const builds: Digest[] = [];
    for (const extra of ["first", "second", "third"]) {
        const sources = { ...SOURCES, "src/extra.ts": `export const name = "${extra}";\n` };
        builds.push(await store.put(await written(sources, environment.crypto)));
        await simulation.jump(simulation.clock.now() + 60_000);
    }
    const [rooted, older, newer] = builds.map((digest) => present(digest, "a build"));
    await store.retain("space", new Set([present(rooted, "the rooted build")]));
    await store.lease(
        present(newer, "the newest build"),
        new Date(simulation.clock.now() + WEEK_MILLISECONDS),
    );

    // evict down to the size of the rooted and the newest build's blobs
    const fitting = (await blobsSize(store, [rooted, newer])) + 1;
    await store.cache.evict(fitting);
    expect({
        stored: await Promise.all(builds.map((digest) => store.contains(digest))),
        isFitting: (await store.blobs.size()) <= fitting,
        isOlderStored: await store.contains(present(older, "the older build")),
    }).toEqual({ stored: [true, false, true], isFitting: true, isOlderStored: false });
});

test("retain a rooted image with every build it lists", async () => {
    // store a library importing an npm unit and another library
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const { crypto } = environment;
    const unit = await store.put(
        await written({ "src/unit.ts": "export {};\n" }, crypto, {}, TINY),
    );
    const heavy = await store.put(await written({ "src/heavy.ts": "export {};\n" }, crypto));
    const middle = await store.put(
        await written(
            { "src/middle.ts": "export {};\n" },
            crypto,
            bunOutputs({ "./client": ["tiny"], "./heavy": ["@example/heavy"] }),
        ),
    );

    // root the image of an application importing the library
    const application = await store.put(
        await written(
            SOURCES,
            crypto,
            bunOutputs({ "./serve": ["@example/middle/client"] }, ["./serve"]),
        ),
    );
    const image = await store.putImage(
        imageOf(application, { "@example/heavy": heavy, "@example/middle": middle, tiny: unit }),
    );
    await store.retain("space-a", new Set([image]));
    await simulation.jump(simulation.clock.now() + LEASE_MILLISECONDS);
    const collected = await collect(store);

    // keep the image
    expect({
        collected,
        stored: await store.has([image, application, middle, heavy, unit]),
    }).toEqual({
        collected: { manifests: [], next: undefined },
        stored: [true, true, true, true, true],
    });
});

test("pull one output of a served build: what its runtime loads and its maps, none of its sources or types", async () => {
    // serve a build whose Bun output has a module, a native addon, its types and a source map, beside its sources and a catalog
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const directory = await temporary(environment.files);
    await using served = await openStore(join(directory, "served"), environment);
    await using pulled = await openStore(join(directory, "pulled"), environment);
    const digest = await served.store.put(
        await written(
            {
                "package.json": "{}\n",
                "src/serve.ts": "export const served = true;\n",
                "locale/de.json": '{"locale":"de"}\n',
                "bun/serve.js": "export {};\n",
                "bun/addon.node": "addon\n",
                "bun/serve.d.ts": "export declare const served: boolean;\n",
                "bun/serve.js.map": '{"version":3}\n',
            },
            environment.crypto,
            bunOutputs({ "./serve": [] }, ["./serve"]),
        ),
    );
    const root = { kind: "manifest", digest } as const;
    const prefix = `/packages/${BuildObject.path(root)}/`;
    const requested: string[] = [];
    const client = new RemoteBuildStore(`https://packages.test${prefix}`, async (request) => {
        requested.push(new URL(request.url).pathname.slice(prefix.length));

        return served.store.fetch(request, MOUNT, { pull: async () => {} });
    });

    // pull the Bun output twice, the second reading nothing, and read which files arrived
    await pulled.store.pull(client, root, ["bun"]);
    await pulled.store.pull(client, root, ["bun"]);
    const reader = await pulled.store.contents(digest);
    const files = (await reader.files()).filter((file) => !file.path.startsWith("manifest/"));
    const stored = await pulled.store.has(files.map((file) => file.digest));
    expect({
        requested,
        isComplete: await pulled.store.contains(digest),
        hasOutput: await pulled.store.containsOutput(digest, "bun"),
        // report the build pulled for Bun, though not whole
        isPulled: [
            await pulled.store.isPulled(root, ["bun"]),
            await pulled.store.isPulled(root, "all"),
        ],
        // hold the manifest's bytes, yet answer an upload that it is incomplete, so a push sends it again
        uploaded: [await pulled.store.has([digest]), await pulled.store.complete([digest])],
        arrived: files.filter((_, index) => stored[index] === true).map((file) => file.path),
    }).toEqual({
        requested: ["read", "read", "read"],
        isComplete: false,
        hasOutput: true,
        isPulled: [true, false],
        uploaded: [[true], [false]],
        arrived: [
            "bun/addon.node",
            "bun/serve.js",
            "bun/serve.js.map",
            "locale/de.json",
            "package.json",
        ],
    });
});

test("refuse an image listing a build the store lacks", async () => {
    // store a build importing a library the store lacks
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { crypto } = environment;
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const lacking = Digest.parse("f".repeat(64));
    const importing = await store.put(
        await written(
            { "bun/import.js": "export {};\n" },
            crypto,
            bunOutputs({ "./import": ["@example/lacking"] }, ["./import"]),
        ),
    );

    // refuse its image
    const image = imageOf(importing, { "@example/lacking": lacking });
    const digest = await BuildImage.digest(image, crypto);
    expect(await refusal(store.putImage(image))).toEqual([
        "NOT_FOUND",
        "MANIFEST_NOT_FOUND",
        `build '${lacking}' of image '${digest}' not found`,
    ]);
});

/** Expire the cache and collect blobs, answering the deleted builds and the next deadline. */
async function collect(
    store: BuildStore,
): Promise<{ readonly manifests: readonly Digest[]; readonly next: number | undefined }> {
    // expire and collect, looking again at the earlier deadline
    const before = await recorded(store);
    const expiring = await store.cache.expire();
    const leased = await store.blobs.collect(0);
    const after = new Set(await recorded(store));
    const ends = [expiring, leased].filter((end) => end !== undefined);

    return {
        manifests: before.filter((digest) => !after.has(digest)),
        next: ends.length === 0 ? undefined : Math.min(...ends),
    };
}

/** List the builds a store records. */
async function recorded(store: BuildStore): Promise<Digest[]> {
    const rows = await store.database.select({ digest: buildManifest.digest }).from(buildManifest);

    return rows.map(({ digest }) => digest).toSorted();
}

test("list a stored build's objects from the store's rows, the same files its manifest distributes", async () => {
    // store a build
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const digest = await store.put(await written(SOURCES, environment.crypto));

    // list its objects through the store's index alone, which has no verb opening a build
    const index = {
        references: (build: Digest) => store.references(build),
        head: (object: Digest) => store.head(object),
        image: (image: Digest) => store.image(image),
    };
    const listed = await BuildObject.of(index, [{ kind: "manifest", digest }]);
    const distributed = await (await store.contents(digest)).references();

    // expect each distinct distributed file's size by its digest, then the manifest
    expect({
        files: Object.fromEntries(
            listed
                .filter((object) => object.kind === "file")
                .map((object) => [object.digest, object.size]),
        ),
        manifests: listed.filter((object) => object.kind === "manifest").map((each) => each.digest),
    }).toEqual({
        files: Object.fromEntries(distributed.map((file) => [file.digest, file.size])),
        manifests: [digest],
    });
});
