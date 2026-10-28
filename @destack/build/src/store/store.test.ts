import { testSimulation } from "../test/world.ts";
import type { Crypto } from "@destack/environment";
import { expect, refusal, test } from "@destack/test";
import type { Environment, FileSystem } from "@destack/environment";
import { join } from "node:path";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { PackageId } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { CallError } from "@destack/object";
import { Digest, found, present } from "@destack/schema";
import {
    type Collection,
    MAPS_REFERRER,
    type PackageAccess,
    PackageClient,
    PackageStore,
    RUNTIME_REFERRER,
} from "./index.ts";
import { packageStoreDatabase } from "./table.ts";

/** The mount the tests serve stores below. */
const MOUNT = new URL("https://packages.test/packages/");
/** The source files of the fixture build. */
const SOURCES = {
    "src/answer.ts": "export const answer = 42;\n",
    "src/index.ts": 'export * from "./answer.ts";\n',
};

/** How long a store leases what it stores for a root to retain: half an hour. */
const LEASE_MILLISECONDS = 30 * 60_000;

/** How long a cache entry retains what it references past its last use: a week. */
const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** A day, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60_000;

/** A minute, in milliseconds. */
const MINUTE_MILLISECONDS = 60_000;

/** The builds the many-builds collection test stores, half of them retained. */
const STRESS_BUILDS = 20;

/** A host granting every read and every upload. */
const UPLOADING: PackageAccess = { authorize: async () => {}, upload: async () => {} };

/** A host granting every read and taking no uploads. */
const READING: PackageAccess = { authorize: async () => {} };

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
    outputs: PackageManifest["outputs"] = {},
    built: PackageManifest["package"] = ANSWER,
): Promise<BuildReader> {
    // describe each source file
    const encoder = new TextEncoder();
    const bytes = new Map(
        Object.entries(sources).map(([path, text]) => [path, encoder.encode(text)]),
    );
    const files = await Promise.all(
        [...bytes].map(([path, contents]) =>
            PackageFile.describe(path, "text/typescript", contents, crypto),
        ),
    );

    // describe the file, dependency and source map lists, and an empty graph
    const index = async (path: string, value: unknown) => {
        const contents = encoder.encode(JSON.stringify(value));
        bytes.set(path, contents);

        return PackageFile.describe(path, "application/json", contents, crypto);
    };
    const manifest = PackageManifest.parse({
        formatVersion: 1,
        package: built,
        language: "typescript",
        lists: {
            dependencies: await index("manifest/dependencies.json", {}),
            files: await index("manifest/files.json", files),
            sourceMaps: await index("manifest/sourceMaps.json", []),
            graph: await index("manifest/graph.json", { modules: {} }),
        },
        outputs,
    });

    return new BuildReader(
        manifest,
        async (path) => new Blob([found(bytes, path)]).stream(),
        crypto,
    );
}

/** Describe a build's Bun output: its exports' imports, its dependencies and its workloads. */
function bunOutputs(
    imports: Readonly<Record<string, readonly string[]>>,
    dependencies: PackageManifest["outputs"][string]["dependencies"],
    entries: readonly string[] = [],
): PackageManifest["outputs"] {
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
                triggers: [],
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
            dependencies,
        },
    };
}

/** Describe a dependency on the tiny npm release, built as a unit at a manifest. */
function npmRelease(manifest: Digest) {
    return {
        kind: "npm" as const,
        package: { name: "tiny", version: "1.0.0" },
        registry: "https://registry.npmjs.org",
        integrity: "sha512-dGlueQ==",
        manifest,
    };
}

/** Describe a dependency on a Destack library's build at a manifest. */
function library(name: string, manifest: Digest) {
    return {
        kind: "destack" as const,
        package: {
            id: PackageId.parse("package-00000000-0000-7000-8000-00000000a002"),
            name,
            version: "2026.9.0",
        },
        manifest,
    };
}

/** Read a stream's bytes. */
async function read(stream: ReadableStream<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** A store over blobs and a database in a directory, closing its database on disposal. */
interface OpenedStore extends AsyncDisposable {
    /** The store. */
    readonly store: PackageStore;
}

/** Open a store over the blobs and the database in a directory, created once. */
async function openStore(
    directory: string,
    environment: Environment<"files" | "process">,
): Promise<OpenedStore> {
    const storage = await LocalBlobStorage.open(join(directory, "files"), environment);
    const database = await connectBunSqlite(join(directory, "store.db"), packageStoreDatabase, {
        environment,
    });
    await database.migrate(packageStoreDatabase.tables);

    return {
        store: new PackageStore({
            blobs: new BlobStore({ storage, database, environment }),
            environment,
        }),
        [Symbol.asyncDispose]: () => database.close(),
    };
}

/** List the digests of a store's blobs in order. */
async function listing(store: PackageStore): Promise<string[]> {
    return (await Array.fromAsync(store.blobs.storage.digests())).toSorted();
}

/** List the blobs of some stored builds: their manifests and every file they list. */
async function blobsOf(store: PackageStore, manifests: readonly Digest[]): Promise<string[]> {
    const files = await Promise.all(
        manifests.map(async (digest) => (await store.contents(digest)).distributed()),
    );

    return [...new Set([...manifests, ...files.flat().map((file) => file.digest)])].toSorted();
}

/** Extract a gzip tarball with the system tar into a directory. */
async function extract(
    archive: Uint8Array,
    directory: string,
    environment: Environment<"files" | "process">,
): Promise<void> {
    const { files } = environment;
    await files.writeFile(join(directory, "archive.tgz"), archive);
    await files.mkdir(join(directory, "extracted"));
    const child = environment.process.spawn(["tar", "-xzf", "archive.tgz", "-C", "extracted"], {
        workingDirectory: directory,
        stderr: "pipe",
    });
    const [code, errors] = await Promise.all([child.exited, new Response(child.stderr).text()]);
    if (code !== 0) {
        throw new Error(`tar failed with ${code}: ${errors}`);
    }
}

test("store a build's files once by digest, read it after reopening, and refuse stored corruption", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    const build = await written(SOURCES, environment.crypto);
    const list = await build.distributed();
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
        expect([await store.put(build), await listing(store)]).toEqual([
            digest,
            [...new Set([...list.map((file) => file.digest), digest])].toSorted(),
        ]);
    }

    // read the same build after reopening the store
    await using opened = await openStore(directory, environment);
    const { store } = opened;
    expect([await store.get(digest), await store.file(answer)]).toEqual([
        build.manifest,
        new TextEncoder().encode(SOURCES["src/answer.ts"]),
    ]);

    // fail reads of stored corruption, and take the stored build by its manifest without rewriting it
    await files.writeFile(
        join(directory, "files", answer.digest),
        new TextEncoder().encode("broken"),
    );
    const corrupt = ["BAD_REQUEST", "FILE_INVALID", "file size mismatch: src/answer.ts"];
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
    const keys = { live: "a".repeat(64), dropped: "b".repeat(64) };
    await store.cache(keys.live, { kind: "build", manifest: live });
    await store.cache(keys.dropped, { kind: "build", manifest: dropped });
    await store.root("space-a", new Set([live]));
    const both = await blobsOf(store, [live, dropped]);

    // collect while the stores' leases and the cache entries retain everything
    const leased = await collect(store);
    const before = await listing(store);

    // collect once the leases end, then use the live entry a day on, and collect once the other went unused for a week
    const start = simulation.clock.now();
    simulation.clock.advance(LEASE_MILLISECONDS);
    const cached = await collect(store);
    simulation.clock.advance(DAY_MILLISECONDS - LEASE_MILLISECONDS);
    await store.cached(keys.live);
    simulation.clock.advance(CACHE_MILLISECONDS - MINUTE_MILLISECONDS);
    const unused = await collect(store);

    // keep exactly the live build's blobs, answering when each collection could delete more
    expect({
        leased,
        before,
        cached,
        unused,
        after: await listing(store),
        entries: [await store.cached(keys.live), await store.cached(keys.dropped)],
    }).toEqual({
        leased: { manifests: [], next: start + LEASE_MILLISECONDS },
        before: both,
        cached: { manifests: [], next: start + CACHE_MILLISECONDS },
        unused: { manifests: [dropped], next: start + DAY_MILLISECONDS + CACHE_MILLISECONDS },
        after: await blobsOf(store, [live]),
        entries: [{ kind: "build", manifest: live }, undefined],
    });
});

test("attach a code index to a build, retain it with the build, and collect it with the build", async () => {
    // store two builds, each with a separate code index attached, rooting the first
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
        await store.receive(file.digest, bytes);
        await store.attach(manifest, file);

        return { manifest, file };
    };
    const live = await attached(SOURCES, "[]");
    const dropped = await attached(
        { ...SOURCES, "src/answer.ts": "export const answer = 43;\n" },
        "[ ]",
    );
    await store.root("space-a", new Set([live.manifest]));

    // refuse a file the store lacks, and read each attached index back
    const missing = { ...live.file, digest: Digest.parse("c".repeat(64)) };
    await expect(store.attach(live.manifest, missing)).rejects.toThrow(
        `build ${live.manifest} lists code.json, which was not uploaded`,
    );
    expect([
        await store.referrer(live.manifest, "code.json"),
        await store.referrer(dropped.manifest, "code.json"),
        await store.referrer(live.manifest, "other.json"),
    ]).toEqual([live.file, dropped.file, undefined]);

    // collect the dropped build once the leases end, its index and its referrer with it
    simulation.clock.advance(LEASE_MILLISECONDS);
    await collect(store);
    expect({
        blobs: await listing(store),
        referrers: [
            await store.referrer(live.manifest, "code.json"),
            await store.referrer(dropped.manifest, "code.json"),
        ],
    }).toEqual({
        blobs: [
            ...new Set([...(await blobsOf(store, [live.manifest])), live.file.digest]),
        ].toSorted(),
        referrers: [live.file, undefined],
    });
});

test("open one reader per stored build, and forget a collected build's reader", async () => {
    // store a build and open it twice
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const digest = await store.put(await written(SOURCES, environment.crypto));
    const [first, second] = [await store.contents(digest), await store.contents(digest)];

    // collect it once its lease ends, then open it again
    simulation.clock.advance(LEASE_MILLISECONDS);
    await collect(store);
    const reopened = await store.contents(digest).then(
        () => "opened",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );

    // share the reader while the build is stored, and refuse it once collected
    expect({ isShared: first === second, reopened }).toEqual({
        isShared: true,
        reopened: "package manifest not found",
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
    const build = await listing(store);
    const until = stored + DAY_MILLISECONDS;
    await store.lease(digest, new Date(until));

    // collect within the lease, and once it ends
    const within = await collect(store);
    const kept = await listing(store);
    simulation.clock.advance(until - simulation.clock.now());
    const ended = await collect(store);
    expect({ within, kept, ended, after: await listing(store) }).toEqual({
        within: { manifests: [], next: stored + LEASE_MILLISECONDS },
        kept: build,
        ended: { manifests: [digest], next: undefined },
        after: [],
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
    simulation.clock.advance(LEASE_MILLISECONDS);
    await collect(store);

    // store the same build again after the collection, which writes every blob back
    const again = await store.put(build);
    await store.root("space-a", new Set([again]));
    simulation.clock.advance(LEASE_MILLISECONDS);
    await collect(store);
    expect({
        again,
        isStored: await store.contains(digest),
        blobs: await listing(store),
        manifest: await store.get(digest),
    }).toEqual({
        again: digest,
        isStored: true,
        blobs: await blobsOf(store, [digest]),
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
    await store.root("space-a", new Set(live));

    // collect once the leases end, keeping exactly the live manifests and the files they list
    simulation.clock.advance(LEASE_MILLISECONDS);
    const collected = await collect(store);
    expect({
        manifests: [...collected.manifests].toSorted(),
        blobs: await listing(store),
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
        (await build.distributed()).find((file) => file.path === "src/answer.ts"),
        "the answer module",
    );
    const bytes = new TextEncoder().encode(SOURCES["src/answer.ts"]);

    // grant bearers of the allowed token reads of the stored build
    const access = {
        async authorize(request: Request, selected: string) {
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
    );
    expect([reader.manifest, await reader.files()]).toEqual([build.manifest, await build.files()]);
    expect(
        await refusal(
            BuildReader.open(
                { manifest: Digest.parse("0".repeat(64)), url },
                fetchPackage,
                environment.crypto,
            ),
        ),
    ).toEqual(["BAD_REQUEST", "FILE_INVALID", "file digest mismatch: manifest.json"]);

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

    // stream the whole build as a tarball of the manifest and every file, answered unchanged by its digest
    const archive = await fetchPackage(`${url}archive`);
    await extract(await read(present(archive.body, "the archive body")), directory, environment);
    const unchanged = await fetchPackage(`${url}archive`, {
        headers: { "if-none-match": `"${digest}.tar.gz"` },
    });
    expect([
        archive.status,
        archive.headers.get("etag"),
        unchanged.status,
        JSON.parse(await files.readTextFile(join(directory, "extracted", "manifest.json"))),
        await files.readTextFile(join(directory, "extracted", "src/answer.ts")),
    ]).toEqual([200, `"${digest}.tar.gz"`, 304, build.manifest, SOURCES["src/answer.ts"]]);

    // release streamed reads when clients disconnect
    await present((await fetchPackage(path)).body, "the file body").cancel();
    await present((await fetchPackage(`${url}archive`)).body, "the archive body").cancel();
});

test("copy a served build into another store by its digest, fetching only the files it lacks", async () => {
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
    const paths: string[] = [];
    const fetchPackage = async (input: URL, options: RequestInit) => {
        paths.push(input.pathname.slice(`/packages/${digest}/`.length));

        return source.fetch(new Request(input, options), MOUNT, { authorize: async () => {} });
    };
    const location = { manifest: digest, url: `https://packages.test/packages/${digest}/` };

    // copy the build, then copy it again, which reads nothing once the store has it
    const before = await target.contains(digest);
    await target.copy(location, fetchPackage);
    const first = paths.splice(0);
    await target.copy(location, fetchPackage);
    expect({
        before,
        after: await target.contains(digest),
        manifest: (await target.contents(digest)).manifest,
        first: first.toSorted(),
        second: paths.toSorted(),
    }).toEqual({
        before: false,
        after: true,
        manifest: build.manifest,
        first: [
            "files/manifest/dependencies.json",
            "files/manifest/files.json",
            "files/manifest/files.json",
            "files/manifest/graph.json",
            "files/manifest/sourceMaps.json",
            "files/src/answer.ts",
            "files/src/index.ts",
            "manifest.json",
        ],
        second: [],
    });
});

test("push a stored build to a package target by its digest, uploading only the files it lacks", async () => {
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
        if (request.method === "PUT") {
            uploads.push(new URL(request.url).pathname);
        }

        return target.fetch(request, MOUNT, UPLOADING);
    };

    // push the build, then push it again, which uploads its manifest alone
    const client = new PackageClient(
        "https://packages.test/packages/",
        accepting,
        environment.crypto,
    );
    await client.push(await source.contents(digest));
    const first = uploads.splice(0);
    await client.push(await source.contents(digest));
    const list = await build.distributed();
    expect({
        manifest: (await target.contents(digest)).manifest,
        first: first.toSorted(),
        second: uploads,
    }).toEqual({
        manifest: build.manifest,
        first: [
            ...new Set(list.map((file) => `/packages/files/${file.digest}`)),
            `/packages/${digest}/manifest.json`,
        ].toSorted(),
        second: [`/packages/${digest}/manifest.json`],
    });
});

test("refuse uploads lacking files, of other bytes, past the limit, or where nobody uploads", async () => {
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

    // stream one byte past the limit in chunks of one MiB without declaring a length
    const chunk = new Uint8Array(1024 * 1024);
    const oversized = new ReadableStream<Uint8Array>({
        start(controller) {
            for (let index = 0; index < 64; index++) {
                controller.enqueue(chunk);
            }
            controller.enqueue(new Uint8Array(1));
            controller.close();
        },
    });

    // refuse each upload with its status and error
    expect([
        await upload(receiving, `${digest}/manifest.json`, manifest, UPLOADING),
        await upload(receiving, `files/${digest}`, new Uint8Array([1]), UPLOADING),
        await upload(receiving, `files/${digest}`, oversized, UPLOADING),
        await upload(receiving, `files/${digest}/extra`, new Uint8Array([1]), UPLOADING),
        await upload(receiving, `${digest}/manifest.json`, manifest, READING),
    ]).toEqual([
        [
            400,
            {
                defined: false,
                code: "BAD_REQUEST",
                status: 400,
                message: `build ${digest} lists manifest/dependencies.json, which was not uploaded`,
                reason: "MANIFEST_BLOB_UNKNOWN",
                domain: "@destack/build",
            },
        ],
        [
            400,
            {
                defined: false,
                code: "BAD_REQUEST",
                status: 400,
                message: `the uploaded file does not match its digest ${digest}`,
                reason: "DIGEST_INVALID",
                domain: "@destack/build",
            },
        ],
        [413, null],
        [404, null],
        [405, null],
    ]);
});

/** Upload a body to a store path as a host decides, returning the status and error. */
async function upload(
    store: PackageStore,
    path: string,
    body: BodyInit,
    access: PackageAccess,
): Promise<[number, unknown]> {
    const response = await store.fetch(
        new Request(`https://packages.test/packages/${path}`, {
            method: "PUT",
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
    simulation.clock.advance(LEASE_MILLISECONDS);
    await store.root("space-a", new Set([first]));
    await store.root("space-b", new Set([second]));
    const both = await store.retained();

    // forget one root and collect its build, leaving the other's retained
    await store.forget("space-a");
    await collect(store);
    expect({
        both: [...both].toSorted(),
        roots: [...(await store.roots()).keys()],
        isFirstStored: await store.contains(first),
        blobs: await listing(store),
    }).toEqual({
        both: [first, second].toSorted(),
        roots: ["space-b"],
        isFirstStored: false,
        blobs: await blobsOf(store, [second]),
    });
});

test("retain a rooted build with every build it depends on, as its outputs record them", async () => {
    // store a library depending on an npm unit build and on another library
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
            bunOutputs(
                { "./client": ["tiny"], "./heavy": ["@example/heavy"] },
                { "@example/heavy": library("@example/heavy", heavy), tiny: npmRelease(unit) },
            ),
        ),
    );

    // root an application depending on the library, then collect once the leases end
    const application = await store.put(
        await written(
            SOURCES,
            crypto,
            bunOutputs(
                { "./serve": ["@example/middle/client"] },
                { "@example/middle": library("@example/middle", middle) },
                ["./serve"],
            ),
        ),
    );
    await store.root("space-a", new Set([application]));
    simulation.clock.advance(LEASE_MILLISECONDS);
    const collected = await collect(store);

    // keep the application, the library and both builds the library depends on
    expect({
        collected,
        stored: await Promise.all(
            [application, middle, heavy, unit].map((digest) => store.contains(digest)),
        ),
        units: await store.units(
            middle,
            PackageId.parse("package-00000000-0000-7000-8000-00000000a002"),
        ),
    }).toEqual({
        collected: { manifests: [], next: undefined },
        stored: [true, true, true, true],
        units: [unit],
    });
});

/** The files of a stored build beside its manifest's lists, by path. */
async function filesOf(store: PackageStore, manifest: Digest): Promise<string[]> {
    const files = await (await store.contents(manifest)).distributed();

    return files
        .map((file) => file.path)
        .filter((path) => !path.startsWith("manifest/"))
        .toSorted();
}

test("derive a build's runtime build once: the output's loaded files, its maps attached to it apart", async () => {
    // store a build whose Bun output has a module, a native addon and a source map
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { crypto } = environment;
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const built = await store.put(
        await written(
            {
                "package.json": "{}\n",
                "bun/serve.js": "export {};\n",
                "bun/addon.node": "addon\n",
                "bun/serve.d.ts": "export {};\n",
                "bun/serve.js.map": "{}\n",
            },
            crypto,
            bunOutputs({ "./serve": [] }, {}, ["./serve"]),
        ),
    );

    // derive it twice at once, and read the runtime build, its maps and its attachment
    const [first, again] = await Promise.all([
        store.attachRuntime(built, "bun"),
        store.attachRuntime(built, "bun"),
    ]);
    const maps = present(await store.referrer(first, MAPS_REFERRER), "the maps build");
    expect({
        again,
        attached: (await store.referrer(built, `${RUNTIME_REFERRER}/bun`))?.digest,
        runtime: await filesOf(store, first),
        maps: await filesOf(store, maps.digest),
    }).toEqual({
        again: first,
        attached: first,
        runtime: ["bun/addon.node", "bun/serve.js", "package.json"],
        maps: ["bun/serve.js.map"],
    });
});

test("list a build that is its own runtime build once among its attached builds", async () => {
    // derive the runtime build of a runtime build, which is itself
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { crypto } = environment;
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const built = await store.put(
        await written(
            { "bun/serve.js": "export {};\n" },
            crypto,
            bunOutputs({ "./serve": [] }, {}, ["./serve"]),
        ),
    );
    const runtime = await store.attachRuntime(built, "bun");
    const again = await store.attachRuntime(runtime, "bun");

    // list the runtime build's attachment to itself once
    const attached = await store.attached(runtime);
    expect({
        again,
        attached: attached.map(({ subject, file }) => [subject, file.digest]),
    }).toEqual({
        again: runtime,
        attached: [[runtime, runtime]],
    });
});

test("retain a rooted build's runtime build and its maps, passing over a dependency the store lacks", async () => {
    // root a served build with its runtime build attached, and a build importing a library the store lacks
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { crypto } = environment;
    await using opened = await openStore(await temporary(environment.files), environment);
    const { store } = opened;
    const served = await store.put(
        await written(
            { "bun/serve.js": "export {};\n", "bun/serve.js.map": "{}\n" },
            crypto,
            bunOutputs({ "./serve": [] }, {}, ["./serve"]),
        ),
    );
    const lacking = Digest.parse("f".repeat(64));
    const importing = await store.put(
        await written(
            { "bun/import.js": "export {};\n" },
            crypto,
            bunOutputs(
                { "./import": ["@example/lacking"] },
                { "@example/lacking": library("@example/lacking", lacking) },
                ["./import"],
            ),
        ),
    );
    await store.root("space-a", new Set([served, importing]));
    const runtime = await store.attachRuntime(served, "bun");
    const maps = present(await store.referrer(runtime, MAPS_REFERRER), "the maps build");

    // retain both builds, the runtime build and its maps, passing over the lacking library
    expect([...(await store.retained())].toSorted()).toEqual(
        [served, runtime, maps.digest, importing].toSorted(),
    );
});

/** Unrecord a store's unretained builds, then delete the blobs they let go, as a partition does. */
async function collect(store: PackageStore): Promise<Collection> {
    // unrecord the builds, then delete the blobs, looking again at the earlier deadline
    const { manifests, next } = await store.collect();
    const leased = await store.blobs.collect();
    const ends = [next, leased].filter((end) => end !== undefined);

    return { manifests, next: ends.length === 0 ? undefined : Math.min(...ends) };
}
