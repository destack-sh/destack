import { testSimulation } from "../test/world.ts";
import type { Crypto } from "@destack/environment";
import { expect, refusal, test } from "@destack/test";
import type { Environment, FileSystem } from "@destack/environment";
import { join } from "node:path";
import { LocalBucket } from "@destack/bucket/bun";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { CallError } from "@destack/object";
import { Digest, found, present } from "@destack/schema";
import { type PackageAccess, PackageClient, PackageStore, Retention } from "./index.ts";

/** The mount the tests serve stores below. */
const MOUNT = new URL("https://packages.test/packages/");
/** The source files of the fixture build. */
const SOURCES = {
    "src/answer.ts": "export const answer = 42;\n",
    "src/index.ts": 'export * from "./answer.ts";\n',
};

/** The interval between two sweeps the sweep tests simulate: an hour. */
const INTERVAL = 60 * 60 * 1000;

/** The builds the many-builds sweep test stores, half of them retained. */
const STRESS_BUILDS = 20;

/** A host granting every read and every upload. */
const UPLOADING: PackageAccess = { authorize: async () => {}, upload: async () => {} };

/** A host granting every read and taking no uploads. */
const READING: PackageAccess = { authorize: async () => {} };

/** Create a temporary directory in a world. */
async function temporary(files: FileSystem): Promise<string> {
    return await files.makeTemporaryDirectory("store-");
}

/** Write a build of some source files in memory, as a builder writes one. */
async function written(
    sources: Readonly<Record<string, string>>,
    crypto: Crypto,
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
        package: {
            id: "package-00000000-0000-7000-8000-00000000a001",
            name: "@example/answer",
            version: "2026.9.0",
        },
        language: "typescript",
        lists: {
            dependencies: await index("manifest/dependencies.json", {}),
            files: await index("manifest/files.json", files),
            sourceMaps: await index("manifest/sourceMaps.json", []),
            graph: await index("manifest/graph.json", { modules: {} }),
        },
        outputs: {},
    });

    return new BuildReader(
        manifest,
        async (path) => new Blob([found(bytes, path)]).stream(),
        crypto,
    );
}

/** Read a stream's bytes. */
async function read(stream: ReadableStream<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** List every key of a bucket in order. */
async function listing(bucket: LocalBucket): Promise<string[]> {
    return (await bucket.list()).files.map((file) => file.key);
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

    // store the build twice, which writes every file once under its digest
    let digest: Digest;
    {
        await using bucket = await LocalBucket.open(
            join(directory, "bucket"),
            "space-test",
            environment,
        );
        const store = new PackageStore(bucket, environment);
        digest = await store.put(build);
        expect([
            await store.put(build),
            (await bucket.list()).files.map((entry) => entry.key),
        ]).toEqual([
            digest,
            [...list.map((file) => `files/${file.digest}`), `manifests/${digest}`].toSorted(),
        ]);
    }

    // read the same build after reopening the bucket
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    expect([await store.get(digest), await store.file(answer)]).toEqual([
        build.manifest,
        new TextEncoder().encode(SOURCES["src/answer.ts"]),
    ]);

    // fail reads of stored corruption, and never overwrite it by storing again
    await bucket.put(`files/${answer.digest}`, "broken");
    expect(await refusal(store.file(answer))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "STORED_CONTENT_DIFFERS",
        "stored file differs: src/answer.ts",
    ]);
    expect(await refusal(store.put(build))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "STORED_CONTENT_DIFFERS",
        `stored object differs: files/${answer.digest}`,
    ]);
});

test("sweep the builds retention drops, the files only they list and the cache entries that list them", async () => {
    // store a live build and a dropped one sharing the index module, each listed by a cache key
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
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
    const before = await bucket.list();

    // spare everything stored since the moment, then mark and drop the build retention leaves out
    await store.sweep(Retention.of([live]), new Date(0), Retention.none, new Date(0));
    expect((await bucket.list()).files.map((file) => file.key)).toEqual(
        before.files.map((file) => file.key),
    );
    const later = new Date(simulation.clock.now() + 1000);
    await store.sweep(Retention.of([live]), later, Retention.none, later);
    await store.sweep(Retention.of([live]), later, Retention.none, later, later);

    // retain exactly the live build's manifest, the files it lists and its cache entry
    const reader = await store.contents(live);
    const distributed = await reader.distributed();
    expect((await bucket.list()).files.map((file) => file.key)).toEqual(
        [
            `cache/${keys.live}`,
            ...new Set(distributed.map((file) => `files/${file.digest}`)),
            `manifests/${live}`,
        ].toSorted(),
    );
    expect(await store.cached(keys.dropped)).toBeUndefined();
});

test("attach a code index to a build, retain it with the build, and sweep it with the build", async () => {
    // store two builds, each with a separate code index attached
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    const attached = async (sources: Readonly<Record<string, string>>, index: string) => {
        const manifest = await store.put(await written(sources, environment.crypto));
        const bytes = new TextEncoder().encode(index);
        const file = await PackageFile.describe(
            "code.json",
            "application/json",
            bytes,
            environment.crypto,
        );
        await store.receive(file.digest, bytes, file.mediaType);
        await store.attach(manifest, file);

        return { manifest, file };
    };
    const live = await attached(SOURCES, "[]");
    const dropped = await attached(
        { ...SOURCES, "src/answer.ts": "export const answer = 43;\n" },
        "[ ]",
    );

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

    // mark and drop the build retention leaves out, its index and its referrer with it
    const later = new Date(simulation.clock.now() + 1000);
    await store.sweep(Retention.of([live.manifest]), later, Retention.none, later);
    await store.sweep(Retention.of([live.manifest]), later, Retention.none, later, later);
    const distributed = await (await store.contents(live.manifest)).distributed();
    expect(await listing(bucket)).toEqual(
        [
            ...new Set([...distributed, live.file].map((file) => `files/${file.digest}`)),
            `manifests/${live.manifest}`,
            `referrers/${live.manifest}/code.json`,
        ].toSorted(),
    );
});

test("sweep an old unretained build but retain the files a recent build shares with it", async () => {
    // store an old build, then a recent one sharing its index module
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    const old = await store.put(await written(SOURCES, environment.crypto));
    simulation.clock.advance(2);
    const moment = new Date(simulation.clock.now());
    const recent = await store.put(
        await written(
            { ...SOURCES, "src/answer.ts": "export const answer = 43;\n" },
            environment.crypto,
        ),
    );

    // mark and drop the old manifest alone, retaining every file the recent manifest lists
    const later = new Date(simulation.clock.now() + 1000);
    await store.sweep(Retention.none, moment, Retention.none, moment);
    await store.sweep(Retention.none, moment, Retention.none, later, later);
    const reader = await store.contents(recent);
    const distributed = await reader.distributed();
    expect({
        isOldStored: await store.contains(old),
        files: (await bucket.list()).files.map((file) => file.key),
    }).toEqual({
        isOldStored: false,
        files: [
            ...new Set(distributed.map((file) => `files/${file.digest}`)),
            `manifests/${recent}`,
        ].toSorted(),
    });
});

test("open one reader per stored build, and forget a swept build's reader", async () => {
    // store a build and open it twice
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    const digest = await store.put(await written(SOURCES, environment.crypto));
    const [first, second] = [await store.contents(digest), await store.contents(digest)];

    // mark and drop it, then open it again
    const later = new Date(simulation.clock.now() + 1000);
    await store.sweep(Retention.none, later, Retention.none, later);
    await store.sweep(Retention.none, later, Retention.none, later, later);
    const reopened = await store.contents(digest).then(
        () => "opened",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );

    // share the reader while the build is stored, and refuse it once swept
    expect({ isShared: first === second, reopened }).toEqual({
        isShared: true,
        reopened: "package manifest not found",
    });
});

test("retain a leased build through sweeps until its lease expires, then mark and delete it", async () => {
    // store a build and lease it for two intervals
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    const digest = await store.put(await written(SOURCES, environment.crypto));
    const build = await listing(bucket);
    const start = simulation.clock.now() + 1000;
    const at = (intervals: number) => new Date(start + intervals * INTERVAL);
    await store.lease(digest, at(2));

    // leave the build unmarked within the lease, mark it once the lease expires, delete it an interval later
    const sweeps = [];
    for (const intervals of [0, 1, 3, 4]) {
        await store.sweep(Retention.none, at(0), Retention.none, at(intervals), at(intervals - 1));
        sweeps.push(await listing(bucket));
    }
    expect(sweeps).toEqual([
        [...build, `leases/${digest}`].toSorted(),
        [...build, `leases/${digest}`].toSorted(),
        [...build, "marks"].toSorted(),
        [],
    ]);
});

test("delete an unretained old build only in the second sweep after a sweep marked it", async () => {
    // store a build, and mark it in a first sweep
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    await store.put(await written(SOURCES, environment.crypto));
    const build = await listing(bucket);
    const start = simulation.clock.now() + 1000;
    const at = (intervals: number) => new Date(start + intervals * INTERVAL);
    await store.sweep(Retention.none, at(0), Retention.none, at(0), at(-1));
    const marks = await present(await bucket.get("marks"), "the marks").json();

    // retain the build through a sweep within the interval, and delete it in the sweep after
    await store.sweep(Retention.none, at(0), Retention.none, at(0.5), at(-0.5));
    const within = await listing(bucket);
    await store.sweep(Retention.none, at(0), Retention.none, at(1), at(0.5));
    expect({ marks, within, after: await listing(bucket) }).toEqual({
        marks: Object.fromEntries(build.map((key) => [key, start])),
        within: [...build, "marks"].toSorted(),
        after: [],
    });
});

test("retain a build a release retains between its mark and the next sweep, and clear its mark", async () => {
    // store a build, and mark it while unretained
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    const digest = await store.put(await written(SOURCES, environment.crypto));
    const build = await listing(bucket);
    const start = simulation.clock.now() + 1000;
    const at = (intervals: number) => new Date(start + intervals * INTERVAL);
    await store.sweep(Retention.none, at(0), Retention.none, at(0), at(-1));

    // leave it unmarked once a release retains it
    await store.sweep(Retention.of([digest]), at(0), Retention.none, at(1), at(0));
    expect(await listing(bucket)).toEqual(build);
});

test("serve a stored build over HTTP after the host authorizes each read", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
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
    await using served = await LocalBucket.open(
        join(directory, "served"),
        "space-test",
        environment,
    );
    await using copied = await LocalBucket.open(
        join(directory, "copied"),
        "space-test",
        environment,
    );
    const source = new PackageStore(served, environment);
    const target = new PackageStore(copied, environment);
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
    await using live = await LocalBucket.open(join(directory, "live"), "space-test", environment);
    await using received = await LocalBucket.open(
        join(directory, "received"),
        "space-test",
        environment,
    );
    const source = new PackageStore(live, environment);
    const target = new PackageStore(received, environment);
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
    await using live = await LocalBucket.open(join(directory, "live"), "space-test", environment);
    await using empty = await LocalBucket.open(join(directory, "empty"), "space-test", environment);
    const source = new PackageStore(live, environment);
    const digest = await source.put(await written(SOURCES, environment.crypto));
    const receiving = new PackageStore(empty, environment);
    const manifest = new Uint8Array(await (await source.manifest(digest)).arrayBuffer());

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

test("retain the union of every root's builds, and forget a root that retains nothing", async () => {
    // store two builds, each retained by a separate root
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    const first = await store.put(await written(SOURCES, environment.crypto));
    const second = await store.put(
        await written({ ...SOURCES, "src/extra.ts": "export {};\n" }, environment.crypto),
    );
    await store.root("space-a", new Set([first]));
    await store.root("space-b", new Set([second]));
    const both = await store.retained();

    // forget one root, leaving the other's builds retained
    await store.root("space-a", new Set());

    expect({ both: [...both].toSorted(), after: [...(await store.retained())] }).toEqual({
        both: [first, second].toSorted(),
        after: [second],
    });
});

test("sweep a store at most once per interval, once among concurrent sweeps, and again after a failed one", async () => {
    // store a build one root retains and one no root retains
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
    const live = await store.put(await written(SOURCES, environment.crypto));
    const dropped = await store.put(
        await written(
            { ...SOURCES, "src/answer.ts": "export const answer = 0;\n" },
            environment.crypto,
        ),
    );
    await store.root("space-a", new Set([live]));
    const start = simulation.clock.now() + 1000;
    const at = (intervals: number) => new Date(start + intervals * INTERVAL);
    const manifests = async () =>
        (await listing(bucket)).filter((key) => key.startsWith("manifests/"));

    // sweep twice at once, marking the dropped build in one of them
    const concurrent = await Promise.all([
        store.sweepIfDue(at(0), at(0), INTERVAL),
        store.sweepIfDue(at(0), at(0), INTERVAL),
    ]);
    const marked = await manifests();

    // leave a sweep within the interval, then fail one past it on a corrupt root
    const within = await store.sweepIfDue(at(0), at(0.5), INTERVAL);
    await bucket.put("roots/space-b", "corrupt");
    const failed = await store.sweepIfDue(at(0), at(1), INTERVAL).then(
        () => undefined,
        (error: unknown) => (error instanceof Error ? [error.name, error.message] : error),
    );
    await bucket.delete("roots/space-b");

    // sweep again at once after the failure, deleting the marked build
    const retried = await store.sweepIfDue(at(0), at(1), INTERVAL);
    expect({
        claimed: concurrent.filter((retained) => retained !== undefined).map((each) => [...each]),
        lost: concurrent.filter((retained) => retained === undefined).length,
        marked,
        within,
        failed,
        retried: retried === undefined ? undefined : [...retried],
        left: await manifests(),
    }).toEqual({
        claimed: [[live]],
        lost: 1,
        marked: [`manifests/${dropped}`, `manifests/${live}`].toSorted(),
        within: undefined,
        failed: ["SyntaxError", 'JSON Parse error: Unexpected identifier "corrupt"'],
        retried: [live],
        left: [`manifests/${live}`],
    });
});

test("sweep many builds sharing a module, sparing exactly the retained ones and the files they list", async () => {
    // store builds sharing the index module, each with a separate answer, retaining the first half
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await temporary(files);
    await using bucket = await LocalBucket.open(
        join(directory, "bucket"),
        "space-test",
        environment,
    );
    const store = new PackageStore(bucket, environment);
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

    // mark and drop the builds retention leaves out
    const later = new Date(simulation.clock.now() + 1000);
    await store.sweep(Retention.of(live), later, Retention.none, later);
    await store.sweep(Retention.of(live), later, Retention.none, later, later);

    // retain exactly the live manifests and the files they list
    const distributed = await Promise.all(
        live.map(async (digest) => (await store.contents(digest)).distributed()),
    );
    expect(await listing(bucket)).toEqual(
        [
            ...new Set(distributed.flat().map((file) => `files/${file.digest}`)),
            ...live.map((digest) => `manifests/${digest}`),
        ].toSorted(),
    );
});
