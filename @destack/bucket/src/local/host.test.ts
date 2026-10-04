import { createHash } from "node:crypto";
import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { setImmediate } from "node:timers/promises";
import { MemoryKeychain } from "@destack/host/keychain";
import { found, present, schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { SweepController } from "../provider/index.ts";
import { StorageError } from "../error/index.ts";
import { S3Credentials, S3Location, S3Server, SignatureV4 } from "../s3/index.ts";
import { LocalBucketHost } from "./host.ts";

/** The bucket the scenarios keep. */
const BUCKET_ID = "bucket-01996ab0-0000-7000-8000-000000000002";

test("serve a host's buckets over S3 by name, presigned with credentials its keychain keeps", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-buckets-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    // generate the host's credentials once
    const keychain = new MemoryKeychain();
    const credentials = await LocalBucketHost.credentials(keychain, "s3/host-1");
    expect([
        await LocalBucketHost.credentials(keychain, "s3/host-1"),
        credentials.accessKeyId.length,
        credentials.secretAccessKey.length,
    ]).toEqual([credentials, 20, 40]);

    // serve the buckets the host keeps, and no other name
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost:4200"),
        region: "local",
        credentials,
    });
    const server = new S3Server({
        region: "local",
        credentials: async (id) => (id === credentials.accessKeyId ? credentials : undefined),
        open: (name) => buckets.named(name),
    });
    const reference = {
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
        bucketId: schema.identifier("bucket").parse("bucket-01996ab0-0000-7000-8000-000000000002"),
    };
    const other = schema.identifier("bucket").parse("bucket-01996ab0-0000-7000-8000-000000000003");
    expect([await buckets.named(other), await buckets.named("files")]).toEqual([
        undefined,
        undefined,
    ]);
    await (await buckets.open(reference, "space-test")).put("notes/a.txt", "first");

    // read the file through a URL presigned at the located endpoint
    const { location } = await buckets.locate(reference, "read");
    const presigned = await new SignatureV4({ region: location.region }).presign(
        new Request(S3Location.url(location, "notes/a.txt").href),
        credentials,
        60,
        Date.now(),
    );
    const response = await server.fetch(presigned);
    expect([new URL(presigned.url).origin, response.status, await response.text()]).toEqual([
        "http://s3.localhost:4200",
        200,
        "first",
    ]);
});

test("keep one set of credentials when two processes open a host's keychain for the first time at once", async () => {
    // generate the first credentials from two openings sharing one keychain
    const keychain = new MemoryKeychain();
    const opened = await Promise.all([
        LocalBucketHost.credentials(keychain, "s3/host-1"),
        LocalBucketHost.credentials(keychain, "s3/host-1"),
    ]);

    // answer the kept credentials to both
    const kept = S3Credentials.parse(JSON.parse(found(keychain.secrets, "s3/host-1")));
    expect(opened).toEqual([kept, kept]);
});

test("sweep a host's open buckets daily, again a minute later while a copy holds the blobs it fetched", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-buckets-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    // keep a file and a blob no file references, as an interrupted write leaves it
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost:4200"),
        region: "local",
        credentials: { accessKeyId: "HOST", secretAccessKey: "host-secret" },
    });
    const bucketId = schema
        .identifier("bucket")
        .parse("bucket-01996ab0-0000-7000-8000-000000000002");
    const bucket = await buckets.open({ bucketId }, "space-test");
    await bucket.put("notes/a.txt", "kept");
    const files = join(buckets.path({ bucketId }), "files");
    await writeFile(join(files, digestOf("abandoned")), "abandoned");
    const sweep = new SweepController(buckets);

    // keep every blob while a copy holds the blobs it fetched, and sweep the abandoned one once released
    const held = await present(bucket.database().blobs, "blobs").hold();
    const busy = [await sweep.reconcile(), (await readdir(files)).toSorted()];
    await held[Symbol.asyncDispose]();
    const swept = [await sweep.reconcile(), await readdir(files)];
    expect({ busy, swept }).toEqual({
        busy: [60_000, [digestOf("abandoned"), digestOf("kept")].toSorted()],
        swept: [24 * 60 * 60_000, [digestOf("kept")]],
    });
});

test("open a bucket again after a failed open", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-buckets-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost:4200"),
        region: "local",
        credentials: { accessKeyId: "HOST", secretAccessKey: "host-secret" },
    });
    const bucket = { bucketId: schema.identifier("bucket").parse(BUCKET_ID) };

    // refuse a bucket never created, then create it in its scope and reuse it
    await expect(buckets.open(bucket)).rejects.toMatchObject({
        code: "NO_SUCH_BUCKET",
        message: `no bucket at ${buckets.path(bucket)}`,
    });
    const created = await buckets.open(bucket, "space-test");
    expect(await buckets.open(bucket)).toBe(created);
});

test("fence a bucket during a transfer: finish the write in flight, refuse every later write across a restart, read on, and lift", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-buckets-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const credentials = { accessKeyId: "HOST", secretAccessKey: "host-secret" };
    const options = {
        directory,
        endpoint: new URL("http://s3.localhost:4200"),
        region: "local",
        credentials,
    };
    const reference = {
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
        bucketId: schema.identifier("bucket").parse(BUCKET_ID),
    };

    // keep a file and an upload with one part, and stream another file's body
    const buckets = new LocalBucketHost(options);
    const bucket = await buckets.open(reference, "space-test");
    await bucket.put("notes/a.txt", "first");
    const upload = await bucket.createMultipartUpload("notes/b.txt");
    const part = await upload.uploadPart(1, "part");
    const pulled = Promise.withResolvers<void>();
    let body: ReadableStreamDefaultController<Uint8Array> | undefined;
    const streamed = new ReadableStream<Uint8Array>({
        start: (controller) => {
            body = controller;
        },
        pull: () => pulled.resolve(),
    });
    const writing = bucket.put("notes/c.txt", streamed);
    await pulled.promise;

    // fence while the write streams, which the fence waits for
    const fencing = buckets.fence(reference);
    const waited = await Promise.race([fencing.then(() => "fenced"), setImmediate("waiting")]);
    present(body, "body").enqueue(new TextEncoder().encode("third"));
    present(body, "body").close();
    await fencing;
    await writing;

    // refuse direct writes, multipart writes, presigned writes and their leases, while reads go on
    const server = new S3Server({
        region: "local",
        credentials: async (id) => (id === credentials.accessKeyId ? credentials : undefined),
        open: (name) => buckets.named(name),
    });
    const { location } = await buckets.locate(reference, "read");
    const signer = new SignatureV4({ region: location.region });
    const url = S3Location.url(location, "notes/d.txt").href;
    const presigned = await signer.presign(
        new Request(url, { method: "PUT" }),
        credentials,
        60,
        Date.now(),
    );
    const put = await server.fetch(new Request(presigned.url, { method: "PUT", body: "fourth" }));
    const fenced = new StorageError("FENCED", "bucket is fenced while a transfer copies it");
    const refusals = await Promise.all(
        [
            bucket.put("notes/d.txt", "fourth"),
            bucket.copy("notes/a.txt", "notes/d.txt"),
            bucket.delete("notes/a.txt"),
            bucket.createMultipartUpload("notes/d.txt"),
            upload.uploadPart(2, "part"),
            upload.complete([part]),
            upload.abort(),
            buckets.locate(reference, "write"),
        ].map((refused) =>
            refused.then(
                () => "accepted",
                (error: unknown) => error,
            ),
        ),
    );
    expect({
        waited,
        written: await (await bucket.get("notes/c.txt"))?.text(),
        refusals,
        put: [put.status, await put.text()],
        read: await (await bucket.get("notes/a.txt"))?.text(),
    }).toEqual({
        waited: "waiting",
        written: "third",
        refusals: Array.from({ length: 8 }, () => fenced),
        put: [
            503,
            `<?xml version="1.0" encoding="UTF-8"?><Error><Code>ServiceUnavailable</Code><Message>${fenced.message}</Message><Resource>/${BUCKET_ID}/notes/d.txt</Resource><RequestId>${put.headers.get("x-amz-request-id")}</RequestId></Error>`,
        ],
        read: "first",
    });

    // keep refusing writes after a restart, until the fence lifts
    await buckets[Symbol.asyncDispose]();
    await using restarted = new LocalBucketHost(options);
    const reopened = await restarted.open(reference);
    const refused = await reopened.put("notes/d.txt", "fourth").catch((error: unknown) => error);
    await restarted.lift(reference);
    await reopened.put("notes/d.txt", "fourth");
    expect([refused, await (await reopened.get("notes/d.txt"))?.text()]).toEqual([
        fenced,
        "fourth",
    ]);
});

/** Name a blob by the SHA-256 digest of its text. */
function digestOf(text: string): string {
    return createHash("sha256").update(text).digest("hex");
}
