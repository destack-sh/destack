import { expect, onTestFinished, test } from "@destack/test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LocalBucket } from "@destack/bucket/local";
import { PackageError } from "@destack/package/error";
import { describeFile } from "@destack/package/file";
import {
    openPackage,
    PackageManifest,
    BuildReader,
    type PackageDistribution,
} from "@destack/package/manifest";
import { ServiceError } from "@destack/service/error";
import { PackageServer, PackageStore } from "./index.ts";

/** The source files of the fixture build. */
const SOURCES = {
    "src/answer.ts": "export const answer = 42;\n",
    "src/index.ts": 'export * from "./answer.ts";\n',
};

/** Create a temporary directory removed when the test finishes. */
async function temporary(): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-store-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    return directory;
}

/** Describe a build of some source files in memory, as a builder distributes one. */
async function distribution(
    sources: Readonly<Record<string, string>>,
): Promise<PackageDistribution> {
    // describe each source file
    const encoder = new TextEncoder();
    const bytes = new Map(
        Object.entries(sources).map(([path, text]) => [path, encoder.encode(text)]),
    );
    const files = await Promise.all(
        [...bytes].map(([path, contents]) => describeFile(path, "text/typescript", contents)),
    );

    // describe the inventory, dependency and source map indexes
    const index = async (path: string, value: unknown) => {
        const contents = encoder.encode(JSON.stringify(value));
        bytes.set(path, contents);

        return describeFile(path, "application/json", contents);
    };
    const manifest = PackageManifest.parse({
        formatVersion: 1,
        package: {
            id: "package-00000000-0000-7000-8000-00000000a001",
            name: "@example/answer",
            version: "2026.9.0",
        },
        language: "typescript",
        dependencies: await index("manifest/dependencies.json", {}),
        descriptions: {},
        outputs: {},
        files: await index("manifest/files.json", files),
        sourceMaps: await index("manifest/sourceMaps.json", []),
    });

    return {
        manifest,
        reader: new BuildReader(manifest, async (path) => bytes.get(path)!),
        open: async (path) => new Blob([bytes.get(path)!]).stream(),
    };
}

/** Read a stream's bytes. */
async function read(stream: ReadableStream<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    return new Uint8Array(await new Response(stream).arrayBuffer());
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
    const build = await distribution(SOURCES);
    const inventory = await build.reader.inventory();
    const answer = inventory.find((file) => file.path === "src/answer.ts")!;

    // store the build twice, which writes every file once under its digest
    let digest: string;
    {
        await using bucket = await LocalBucket.open(join(directory, "bucket"));
        const store = new PackageStore(bucket);
        digest = await store.put(build);
        expect([
            await store.put(build),
            (await bucket.list()).files.map((entry) => entry.key),
        ]).toEqual([
            digest,
            [...inventory.map((file) => `files/${file.digest}`), `manifests/${digest}`].sort(),
        ]);
    }

    // read the same build after reopening the bucket
    await using bucket = await LocalBucket.open(join(directory, "bucket"));
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

test("serve a stored build over HTTP after the host authorizes each read", async () => {
    const directory = await temporary();
    await using bucket = await LocalBucket.open(join(directory, "bucket"));
    const store = new PackageStore(bucket);
    const build = await distribution(SOURCES);
    const digest = await store.put(build);
    const answer = (await build.reader.inventory()).find((file) => file.path === "src/answer.ts")!;
    const bytes = new TextEncoder().encode(SOURCES["src/answer.ts"]);

    // grant bearers of the allowed token reads of the stored build, expiring when the host says so
    let isExpired = false;
    const server = new PackageServer(new URL("https://packages.test/packages/"), store, {
        async authorize(request, selected) {
            if (request.headers.get("authorization") !== "Bearer allowed" || selected !== digest) {
                throw new ServiceError("UNAUTHORIZED", { message: "package access denied" });
            }

            return isExpired ? { expiresAt: 0 } : {};
        },
    });
    const url = `https://packages.test/packages/${digest}/`;
    const fetchPackage = async (input: string | URL, options?: RequestInit) => {
        const request = new Request(String(input), options);
        request.headers.set("authorization", "Bearer allowed");

        return server.fetch(request);
    };

    // open the build as a remote package, refusing another manifest digest
    const reader = await openPackage({ manifest: digest, url }, { fetch: fetchPackage });
    expect([reader.manifest, await reader.files()]).toEqual([
        build.manifest,
        await build.reader.files(),
    ]);
    await expect(
        openPackage({ manifest: "0".repeat(64), url }, { fetch: fetchPackage }),
    ).rejects.toEqual(new PackageError("INVALID_FILE", "file digest mismatch: manifest.json"));

    // authorize before serving partial bodies or answering cache conditions
    const path = `${url}files/src/answer.ts`;
    const full = await fetchPackage(path);
    const partial = await fetchPackage(path, { headers: { range: "bytes=7-11" } });
    const cached = await fetchPackage(path, { headers: { "if-none-match": `"${answer.digest}"` } });
    const denied = await server.fetch(
        new Request(path, { headers: { "if-none-match": `"${answer.digest}"` } }),
    );
    const head = await fetchPackage(path, { method: "HEAD" });
    const missing = await fetchPackage(`${url}files/missing.ts`);
    expect({
        full: await read(full.body!),
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
    await extract(await read(archive.body!), directory);
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
    await (await fetchPackage(path)).body!.cancel();
    await (await fetchPackage(`${url}archive`)).body!.cancel();
});
