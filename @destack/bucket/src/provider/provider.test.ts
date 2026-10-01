import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { identifier } from "@destack/schema";
import { Recipient } from "@destack/resource";
import { expect, onTestFinished, test } from "@destack/test";
import { S3Server } from "../s3/index.ts";
import { LocalBucketHost } from "../local/index.ts";
import { localBucketProvider } from "./provider.ts";

/** The bucket resource both hosts keep. */
const record = {
    id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000002"),
    scope: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
    kind: "bucket" as const,
    spec: {},
    reference: null,
};

/** The credentials both hosts presign with. */
const CREDENTIALS = { accessKeyId: "TRANSFER", secretAccessKey: "transfer-secret" };

/** Open a temporary directory removed after the test. */
async function temporary(): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-provider-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    return directory;
}

test("provision a bucket once and destroy it with its files", async () => {
    const directory = await temporary();
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: CREDENTIALS,
    });
    const provider = localBucketProvider(buckets);

    // provision twice under one reference, keeping a file
    const provision = await provider.provision(record);
    expect(await provider.provision(record)).toEqual(provision);
    const bucket = await buckets.open({ resourceId: record.id });
    await bucket.put("notes/a.txt", "first");
    expect(await readdir(directory)).toEqual([record.id]);

    // destroy the bucket's directory with its files
    await provider.destroy({ ...record, ...provision });
    expect(await readdir(directory)).toEqual([]);
});

test("copy a bucket's files to another host through presigned URLs, keeping exactly the source's files and their identity", async () => {
    // serve the source host's buckets over S3
    const directory = await temporary();
    const server = Bun.serve({ port: 0, fetch: (request) => s3.fetch(request) });
    onTestFinished(() => server.stop(true));
    await using sources = new LocalBucketHost({
        directory: join(directory, "source"),
        endpoint: new URL(server.url),
        region: "local",
        credentials: CREDENTIALS,
    });
    await using targets = new LocalBucketHost({
        directory: join(directory, "target"),
        endpoint: new URL("http://target.invalid"),
        region: "local",
        credentials: CREDENTIALS,
    });
    const s3 = new S3Server({
        region: "local",
        credentials: async (id) => (id === CREDENTIALS.accessKeyId ? CREDENTIALS : undefined),
        open: (name) => sources.named(name),
    });
    const source = localBucketProvider(sources);
    const target = localBucketProvider(targets);

    // keep files on the source, one of them in parts, and a stale one on the target
    const from = { ...record, ...(await source.provision(record)) };
    const to = { ...record, ...(await target.provision(record)) };
    const files = await sources.open({ resourceId: record.id });
    await files.put("notes/a.txt", "first", {
        httpMetadata: { contentType: "text/plain", cacheExpiry: new Date(0) },
        customMetadata: { author: "ada" },
    });
    await files.put("b.bin", new Uint8Array([1, 2, 3]));
    const upload = await files.createMultipartUpload("c.bin");
    await upload.complete([await upload.uploadPart(1, new Uint8Array([7, 8]))]);
    const copied = await targets.open({ resourceId: record.id });
    await copied.put("stale.txt", "old");

    // copy live, change a file, then copy again once fenced, from the live cursor
    const recipient = await Recipient.generate();
    const desired: never[] = [];
    let cursor: string | undefined;
    const pass = async (stage: "live" | "fenced") => {
        for await (const chunk of source.export(
            { record: from, desired, recipient, stage },
            cursor,
            AbortSignal.timeout(4000),
        )) {
            await target.import({ record: to, desired, recipient, stage }, chunk);
            cursor = chunk.cursor;
        }
    };
    await pass("live");
    await files.put("b.bin", new Uint8Array([4]));
    await pass("fenced");

    // keep the source's files with their bytes, metadata, entity tags, versions and upload times, and none else
    const listed = await copied.list({ include: ["httpMetadata", "customMetadata"] });
    const described = async (bucket: typeof files, key: string) => {
        const file = (await bucket.get(key))!;

        return [
            file.key,
            file.etag,
            file.version,
            file.uploaded,
            file.httpMetadata,
            file.customMetadata,
            [...new Uint8Array(await file.arrayBuffer())],
        ];
    };
    expect(await Promise.all(listed.files.map((file) => described(copied, file.key)))).toEqual(
        await Promise.all(["b.bin", "c.bin", "notes/a.txt"].map((key) => described(files, key))),
    );
});

test("refuse transferring a file encrypted with a customer key", async () => {
    const directory = await temporary();
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: CREDENTIALS,
    });
    const provider = localBucketProvider(buckets);
    const provisioned = { ...record, ...(await provider.provision(record)) };
    const bucket = await buckets.open({ resourceId: record.id });
    await bucket.put("sealed.txt", "hidden", { ssecKey: "1".repeat(64) });

    // refuse naming the file in a chunk
    const copy = {
        record: provisioned,
        desired: [],
        recipient: await Recipient.generate(),
        stage: "live" as const,
    };
    await expect(
        provider.export(copy, undefined, AbortSignal.timeout(4000))[Symbol.asyncIterator]().next(),
    ).rejects.toMatchObject({
        code: "UNSUPPORTED",
        message: "transferring sealed.txt, encrypted with a customer key, is not supported",
    });
});
