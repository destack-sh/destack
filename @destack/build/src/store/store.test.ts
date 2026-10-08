import { expect, onTestFinished, test } from "@destack/test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LocalBucket } from "@destack/bucket/bun";
import { PackageError } from "@destack/package/error";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { ServiceError } from "@destack/service/error";
import { found, present } from "@destack/schema";
import { type PackageAccess, PackageClient, PackageServer, PackageStore } from "./index.ts";

/** The source files of the fixture build. */
const SOURCES = {
    "src/answer.ts": "export const answer = 42;\n",
    "src/index.ts": 'export * from "./answer.ts";\n',
};

/** The interval between two sweeps the sweep tests simulate: an hour. */
const INTERVAL = 60 * 60 * 1000;

/** A host granting every read and every upload. */
const UPLOADING: PackageAccess = { authorize: async () => ({}), upload: async () => {} };

/** A host granting every read and taking no uploads. */
const READING: PackageAccess = { authorize: async () => ({}) };

/** Create a temporary directory removed when the test finishes. */
async function temporary(): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-store-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    return directory;
}

/** Write a build of some source files in memory, as a builder writes one. */
async function written(sources: Readonly<Record<string, string>>): Promise<BuildReader> {
    // describe each source file
    const encoder = new TextEncoder();
    const bytes = new Map(
        Object.entries(sources).map(([path, text]) => [path, encoder.encode(text)]),
    );
    const files = await Promise.all(
        [...bytes].map(([path, contents]) =>
            PackageFile.describe(path, "text/typescript", contents),
        ),
    );

    // describe the file, dependency and source map lists, and an empty graph
    const index = async (path: string, value: unknown) => {
        const contents = encoder.encode(JSON.stringify(value));
        bytes.set(path, contents);

        return PackageFile.describe(path, "application/json", contents);
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

    return new BuildReader(manifest, async (path) => new Blob([found(bytes, path)]).stream());
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
async function extract(archive: Uint8Array, directory: string): Promise<void> {
    await writeFile(join(directory, "archive.tgz"), archive);
    await mkdir(join(directory, "extracted"));
    const child = Bun.spawn(["tar", "-xzf", "archive.tgz", "-C", "extracted"], {
        cwd: directory,
        stderr: "pipe",
    });
    const [code, errors] = await Promise.all([child.exited, new Response(child.stderr).text()]);
    if (code !== 0) {
        throw new Error(`tar failed with ${code}: ${errors}`);
    }
}

test("store a build's files once by digest, read it after reopening, and refuse stored corruption", async () => {
    const directory = await temporary();
    const build = await written(SOURCES);
    const list = await build.distributed();
    const answer = present(
        list.find((file) => file.path === "src/answer.ts"),
        "the answer module",
    );

    // store the build twice, which writes every file once under its digest
    let digest: string;
    {
        await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
        const store = new PackageStore(bucket);
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
    await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
    const store = new PackageStore(bucket);
    expect([await store.get(digest), await store.file(answer)]).toEqual([
        build.manifest,
        new TextEncoder().encode(SOURCES["src/answer.ts"]),
    ]);

    // fail reads of stored corruption, and never overwrite it by storing again
    await bucket.put(`files/${answer.digest}`, "broken");
    await expect(store.file(answer)).rejects.toEqual(
        new ServiceError("INTERNAL_SERVER_ERROR", {
            message: "stored file differs: src/answer.ts",
        }),
    );
    await expect(store.put(build)).rejects.toEqual(
        new ServiceError("INTERNAL_SERVER_ERROR", {
            message: `stored object differs: files/${answer.digest}`,
        }),
    );
});

test("sweep the builds retention drops, the files only they list and the cache entries naming them", async () => {
    // store a kept build and a dropped one sharing the index module, each named by a cache key
    const directory = await temporary();
    await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
    const store = new PackageStore(bucket);
    const kept = await store.put(await written(SOURCES));
    const dropped = await store.put(
        await written({ ...SOURCES, "src/answer.ts": "export const answer = 43;\n" }),
    );
    const keys = { kept: "a".repeat(64), dropped: "b".repeat(64) };
    await store.cache(keys.kept, { kind: "build", manifest: kept });
    await store.cache(keys.dropped, { kind: "build", manifest: dropped });
    const before = await bucket.list();

    // spare everything stored since the moment, then mark and drop the build retention leaves out
    await store.sweep(new Set([kept]), new Date(0));
    expect((await bucket.list()).files.map((file) => file.key)).toEqual(
        before.files.map((file) => file.key),
    );
    const later = new Date(Date.now() + 1000);
    await store.sweep(new Set([kept]), later);
    await store.sweep(new Set([kept]), later, new Set(), later, later);

    // keep exactly the kept build's manifest, the files it lists and its cache entry
    const reader = await store.contents(kept);
    const files = await reader.distributed();
    expect((await bucket.list()).files.map((file) => file.key)).toEqual(
        [
            `cache/${keys.kept}`,
            ...new Set(files.map((file) => `files/${file.digest}`)),
            `manifests/${kept}`,
        ].toSorted(),
    );
    expect(await store.cached(keys.dropped)).toBeUndefined();
});

test("sweep an old unretained build but keep the files a recent build shares with it", async () => {
    // store an old build, then a recent one sharing its index module
    const directory = await temporary();
    await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
    const store = new PackageStore(bucket);
    const old = await store.put(await written(SOURCES));
    await Bun.sleep(2);
    const moment = new Date();
    const recent = await store.put(
        await written({ ...SOURCES, "src/answer.ts": "export const answer = 43;\n" }),
    );

    // mark and drop the old manifest alone, keeping every file the recent manifest lists
    const later = new Date(Date.now() + 1000);
    await store.sweep(new Set(), moment);
    await store.sweep(new Set(), moment, new Set(), later, later);
    const reader = await store.contents(recent);
    const files = await reader.distributed();
    expect({
        isOldKept: await store.contains(old),
        files: (await bucket.list()).files.map((file) => file.key),
    }).toEqual({
        isOldKept: false,
        files: [
            ...new Set(files.map((file) => `files/${file.digest}`)),
            `manifests/${recent}`,
        ].toSorted(),
    });
});

test("keep a leased build through sweeps until its lease expires, then mark and delete it", async () => {
    // store a build and lease it for two intervals
    const directory = await temporary();
    await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
    const store = new PackageStore(bucket);
    const digest = await store.put(await written(SOURCES));
    const build = await listing(bucket);
    const start = Date.now() + 1000;
    const at = (intervals: number) => new Date(start + intervals * INTERVAL);
    await store.lease(digest, at(2));

    // keep the build unmarked within the lease, then mark it once the lease expires, and delete it an interval later
    const sweeps = [];
    for (const intervals of [0, 1, 3, 4]) {
        await store.sweep(new Set(), at(0), new Set(), at(intervals), at(intervals - 1));
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
    const directory = await temporary();
    await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
    const store = new PackageStore(bucket);
    await store.put(await written(SOURCES));
    const build = await listing(bucket);
    const start = Date.now() + 1000;
    const at = (intervals: number) => new Date(start + intervals * INTERVAL);
    await store.sweep(new Set(), at(0), new Set(), at(0), at(-1));
    const marks = await present(await bucket.get("marks"), "the marks").json();

    // keep the build through a sweep within the interval, and delete it in the sweep after
    await store.sweep(new Set(), at(0), new Set(), at(0.5), at(-0.5));
    const within = await listing(bucket);
    await store.sweep(new Set(), at(0), new Set(), at(1), at(0.5));
    expect({ marks, within, after: await listing(bucket) }).toEqual({
        marks: Object.fromEntries(build.map((key) => [key, start])),
        within: [...build, "marks"].toSorted(),
        after: [],
    });
});

test("keep a build retained between its mark and the next sweep, and clear its mark", async () => {
    // store a build, and mark it while unretained
    const directory = await temporary();
    await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
    const store = new PackageStore(bucket);
    const digest = await store.put(await written(SOURCES));
    const build = await listing(bucket);
    const start = Date.now() + 1000;
    const at = (intervals: number) => new Date(start + intervals * INTERVAL);
    await store.sweep(new Set(), at(0), new Set(), at(0), at(-1));

    // keep it unmarked once a release retains it
    await store.sweep(new Set([digest]), at(0), new Set(), at(1), at(0));
    expect(await listing(bucket)).toEqual(build);
});

test("serve a stored build over HTTP after the host authorizes each read", async () => {
    const directory = await temporary();
    await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
    const store = new PackageStore(bucket);
    const build = await written(SOURCES);
    const digest = await store.put(build);
    const answer = present(
        (await build.distributed()).find((file) => file.path === "src/answer.ts"),
        "the answer module",
    );
    const bytes = new TextEncoder().encode(SOURCES["src/answer.ts"]);

    // grant bearers of the allowed token reads of the stored build, expiring when the host says so
    let isExpired = false;
    const server = new PackageServer(new URL("https://packages.test/packages/"), store);
    const access = {
        async authorize(request: Request, selected: string) {
            if (request.headers.get("authorization") !== "Bearer allowed" || selected !== digest) {
                throw new ServiceError("UNAUTHORIZED", { message: "package access denied" });
            }

            return isExpired ? { expiresAt: 0 } : {};
        },
    };
    const url = `https://packages.test/packages/${digest}/`;
    const fetchPackage = async (input: string | URL, options?: RequestInit) => {
        const request = new Request(String(input), options);
        request.headers.set("authorization", "Bearer allowed");

        return server.fetch(request, access);
    };

    // open the build as a remote package, refusing another manifest digest
    const reader = await BuildReader.open({ manifest: digest, url }, fetchPackage);
    expect([reader.manifest, await reader.files()]).toEqual([build.manifest, await build.files()]);
    await expect(BuildReader.open({ manifest: "0".repeat(64), url }, fetchPackage)).rejects.toEqual(
        new PackageError("INVALID_FILE", "file digest mismatch: manifest.json"),
    );

    // authorize before serving partial bodies or answering cache conditions
    const path = `${url}files/src/answer.ts`;
    const full = await fetchPackage(path);
    const partial = await fetchPackage(path, { headers: { range: "bytes=7-11" } });
    const cached = await fetchPackage(path, { headers: { "if-none-match": `"${answer.digest}"` } });
    const denied = await server.fetch(
        new Request(path, { headers: { "if-none-match": `"${answer.digest}"` } }),
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
        denied: [401, { error: "UNAUTHORIZED", message: "package access denied" }],
        missing: 404,
        head: [200, String(bytes.length), ""],
    });
    isExpired = true;
    const expired = await fetchPackage(`${url}manifest.json`);
    isExpired = false;

    // stream the whole build as a tarball of the manifest and every file, answered unchanged by its digest
    const archive = await fetchPackage(`${url}archive`);
    await extract(await read(present(archive.body, "the archive body")), directory);
    const unchanged = await fetchPackage(`${url}archive`, {
        headers: { "if-none-match": `"${digest}.tar.gz"` },
    });
    expect([
        expired.status,
        archive.status,
        archive.headers.get("etag"),
        unchanged.status,
        JSON.parse(await readFile(join(directory, "extracted", "manifest.json"), "utf8")),
        await readFile(join(directory, "extracted", "src/answer.ts"), "utf8"),
    ]).toEqual([410, 200, `"${digest}.tar.gz"`, 304, build.manifest, SOURCES["src/answer.ts"]]);

    // release streamed reads when clients disconnect
    await present((await fetchPackage(path)).body, "the file body").cancel();
    await present((await fetchPackage(`${url}archive`)).body, "the archive body").cancel();
});

test("copy a served build into another store by its digest, fetching only the files it lacks", async () => {
    // serve a stored build, recording each path read
    const directory = await temporary();
    await using served = await LocalBucket.open(join(directory, "served"), "space-test");
    await using copied = await LocalBucket.open(join(directory, "copied"), "space-test");
    const source = new PackageStore(served);
    const target = new PackageStore(copied);
    const build = await written(SOURCES);
    const digest = await source.put(build);
    const server = new PackageServer(new URL("https://packages.test/packages/"), source);
    const paths: string[] = [];
    const fetchPackage = async (input: URL, options: RequestInit) => {
        paths.push(input.pathname.slice(`/packages/${digest}/`.length));

        return server.fetch(new Request(input, options), { authorize: async () => ({}) });
    };
    const location = { manifest: digest, url: `https://packages.test/packages/${digest}/` };

    // copy the build, then copy it again, which reads its manifest and its list only
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
        second: ["files/manifest/files.json", "manifest.json"],
    });
});

test("push a kept build to a package server by its digest, uploading only the files it lacks", async () => {
    // keep a build, and serve another store that accepts uploads
    const directory = await temporary();
    await using kept = await LocalBucket.open(join(directory, "kept"), "space-test");
    await using received = await LocalBucket.open(join(directory, "received"), "space-test");
    const source = new PackageStore(kept);
    const target = new PackageStore(received);
    const build = await written(SOURCES);
    const digest = await source.put(build);
    const server = new PackageServer(new URL("https://packages.test/packages/"), target);
    const uploads: string[] = [];
    const accepting = async (request: Request) => {
        if (request.method === "PUT") {
            uploads.push(new URL(request.url).pathname);
        }

        return server.fetch(request, UPLOADING);
    };

    // push the build, then push it again, which uploads its manifest alone
    const client = new PackageClient("https://packages.test/packages/", accepting);
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
    // keep a build, and serve an empty store
    const directory = await temporary();
    await using kept = await LocalBucket.open(join(directory, "kept"), "space-test");
    await using empty = await LocalBucket.open(join(directory, "empty"), "space-test");
    const source = new PackageStore(kept);
    const digest = await source.put(await written(SOURCES));
    const server = new PackageServer(
        new URL("https://packages.test/packages/"),
        new PackageStore(empty),
    );
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
        await upload(server, `${digest}/manifest.json`, manifest, UPLOADING),
        await upload(server, `files/${digest}`, new Uint8Array([1]), UPLOADING),
        await upload(server, `files/${digest}`, oversized, UPLOADING),
        await upload(server, `files/${digest}/extra`, new Uint8Array([1]), UPLOADING),
        await upload(server, `${digest}/manifest.json`, manifest, READING),
    ]).toEqual([
        [
            409,
            {
                error: "CONFLICT",
                message: `build ${digest} lists manifest/dependencies.json, which was not uploaded`,
            },
        ],
        [
            400,
            {
                error: "BAD_REQUEST",
                message: `the uploaded file does not match its digest ${digest}`,
            },
        ],
        [413, null],
        [404, null],
        [405, null],
    ]);
});

/** Upload a body to a server's path as a host decides, answering the status and the error it reports. */
async function upload(
    server: PackageServer,
    path: string,
    body: BodyInit,
    access: PackageAccess,
): Promise<[number, unknown]> {
    const response = await server.fetch(
        new Request(`https://packages.test/packages/${path}`, {
            method: "PUT",
            body,
            headers: { "content-type": "application/octet-stream" },
        }),
        access,
    );
    const text = await response.text();

    return [response.status, text === "" ? null : JSON.parse(text)];
}
