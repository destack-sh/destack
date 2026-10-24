import { createHash } from "node:crypto";
import type { Environment, HostCapability } from "@destack/environment";
import { join } from "node:path";
import { expect, test, refusal } from "@destack/test";
import { type Bucket, BucketFile } from "../bucket/index.ts";
import { BucketError } from "../error/index.ts";
import { LocalBucket } from "../bun/index.ts";
import { S3Client } from "./client.ts";
import { S3Server } from "./server.ts";
import { Simulation } from "@destack/environment/simulation";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The access key the host authorized the client's requests for. */
const ACCESS_KEY_ID = "space-test";

/** The credentials the host knows the access key's signatures by. */
const CREDENTIALS = { accessKeyId: ACCESS_KEY_ID, secretAccessKey: "secret" };

/** The clients a roundtrip runs through: one the host authorized, and one signing every request. */
const CLIENTS = ["authorized", "signed"] as const;

/** A customer key encrypting a file, as 64 hexadecimal digits. */
const CUSTOMER_KEY = "07".repeat(32);

/** Describe a file as plain values, its upload time in whole seconds as HTTP sends it. */
function describe(file: BucketFile | null) {
    if (file === null) {
        return null;
    }

    return {
        key: file.key,
        version: file.version,
        size: file.size,
        etag: file.etag,
        uploaded: Math.floor(file.uploaded.getTime() / 1000),
        httpMetadata: file.httpMetadata,
        customMetadata: file.customMetadata,
        digest: file.digest.toJSON(),
        storageClass: file.storageClass,
        ssecKeyMd5: file.ssecKeyMd5,
        retainUntil: file.retainUntil,
    };
}

/** Read a file's description, range and text, null for a missing file. */
async function read(bucket: Bucket, key: string, options: Parameters<Bucket["get"]>[1] = {}) {
    const file = await bucket.get(key, options);

    return file === null
        ? null
        : { file: describe(file), range: file.range, text: await file.text() };
}

/** Run an operation and describe its refusal, with a failed precondition's current file. */
async function outcome(operation: Promise<unknown>) {
    try {
        await operation;

        return "done";
    } catch (error) {
        if (BucketError.is(error, "PRECONDITION_UNMET")) {
            const { current } = error.data;

            return {
                code: error.reason,
                current: describe(current === null ? null : BucketFile.read(current)),
            };
        } else if (BucketError.is(error)) {
            return { code: error.reason, message: error.message };
        }
        throw error;
    }
}

/** List the documents by limit, cursor, start key and delimiter. */
async function listings(bucket: Bucket) {
    const first = await bucket.list({ prefix: "documents/", limit: 2 });

    return [
        first,
        first.truncated ? await bucket.list({ prefix: "documents/", cursor: first.cursor }) : null,
        await bucket.list({ prefix: "documents/", startAfter: "documents/locked.txt" }),
        await bucket.list({ delimiter: "/" }),
    ];
}

/** Serve a fresh local bucket over S3, authenticating signed requests by the known credentials. */
async function serveBucket(world: Environment<HostCapability>) {
    const directory = await world.files.makeTemporaryDirectory("s3-client-");
    const bucket = await LocalBucket.open(join(directory, "files"), ACCESS_KEY_ID, world);
    const server = new S3Server({
        region: "auto",
        environment: world,
        credentials: async (accessKeyId) =>
            accessKeyId === ACCESS_KEY_ID ? CREDENTIALS : undefined,
        open: async (name, accessKeyId) =>
            name === "files" && accessKeyId === ACCESS_KEY_ID ? bucket : undefined,
    });

    return { bucket, server };
}

/** Serve S3 on the world's network, answering the bucket's location. */
function listen(server: S3Server, world: Environment<HostCapability>) {
    const served = world.server.http({
        hostname: "s3.test",
        port: 443,
        fetch: (request) => server.fetch(request),
    });

    return {
        endpoint: served.url,
        bucket: "files",
        region: "auto",
    };
}

/** Open a client of a kind: authorized by the host in process, or signing over a local port. */
function connect(
    kind: (typeof CLIENTS)[number],
    server: S3Server,
    world: Environment<HostCapability>,
): S3Client {
    // send in process as the access key the host authorized
    if (kind === "authorized") {
        return new S3Client(
            "https://s3.test/files/",
            (request) => server.serve(request, ACCESS_KEY_ID),
            world.crypto,
        );
    }
    // sign every request over a local port
    else {
        return S3Client.of({ location: listen(server, world), credentials: CREDENTIALS }, world);
    }
}

test.for(CLIENTS)(
    "roundtrip every bucket operation through S3 as a %s client, answering as the bucket answers directly",
    async (kind) => {
        // serve a local bucket over S3 to the client
        await using simulation = new Simulation({ seed: 1, start: START });
        const world = simulation.environment("host");
        const served = await serveBucket(world);
        await using bucket = served.bucket;
        const client = connect(kind, served.server, world);

        // store files with metadata, a storage class, a checksum and a customer key, as the bucket then keeps them
        const body = "Hello, world!";
        const expiry = new Date(Date.UTC(2027, 0, 1));
        const stored = await client.put("documents/readme.txt", body, {
            httpMetadata: {
                contentType: "text/plain",
                cacheControl: "no-cache",
                cacheExpiry: expiry,
            },
            customMetadata: { author: "fixture" },
            storageClass: "InfrequentAccess",
            sha256: createHash("sha256").update(body).digest("hex"),
        });
        const hashed = await client.put("documents/hashed.txt", body, {
            sha1: createHash("sha1").update(body).digest(),
            md5: createHash("md5").update(body).digest("hex"),
        });
        const sealed = await client.put("documents/sealed.txt", body, { ssecKey: CUSTOMER_KEY });
        const locked = await client.put("documents/locked.txt", body, {
            retainUntil: new Date(world.clock.now() + 3_600_000),
        });
        const empty = await client.put("documents/empty.txt", null);
        expect([stored, hashed, sealed, locked, empty].map(describe)).toEqual(
            await Promise.all(
                [
                    "documents/readme.txt",
                    "documents/hashed.txt",
                    "documents/sealed.txt",
                    "documents/locked.txt",
                    "documents/empty.txt",
                ].map(async (key) => describe(await bucket.head(key))),
            ),
        );

        // read metadata, whole bodies and ranges, with and without the customer key
        const reads = async (on: Bucket) => [
            describe(await on.head("documents/readme.txt")),
            describe(await on.head("documents/missing.txt")),
            await read(on, "documents/readme.txt"),
            await read(on, "documents/readme.txt", { range: { offset: 7, length: 5 } }),
            await read(on, "documents/readme.txt", { range: { suffix: 6 } }),
            await read(on, "documents/readme.txt", { range: { offset: 7 } }),
            await read(on, "documents/sealed.txt", { ssecKey: CUSTOMER_KEY }),
            await read(on, "documents/missing.txt"),
            await outcome(on.get("documents/sealed.txt")),
            await outcome(on.get("documents/readme.txt", { range: { offset: 100 } })),
        ];
        expect(await reads(client)).toEqual(await reads(bucket));

        // refuse failed preconditions of reads and writes with the file they met, and pass held ones
        const etag = stored.etag;
        const second = new Date(Math.floor(world.clock.now() / 1000) * 1000 - 60_000);
        const conditions = async (on: Bucket) => [
            await outcome(on.get("documents/readme.txt", { onlyIf: { etagMatches: "other" } })),
            await outcome(on.get("documents/readme.txt", { onlyIf: { etagDoesNotMatch: etag } })),
            await outcome(on.get("documents/readme.txt", { onlyIf: { uploadedBefore: second } })),
            await read(on, "documents/readme.txt", { onlyIf: { etagMatches: etag } }),
            await read(on, "documents/readme.txt", { onlyIf: { uploadedAfter: second } }),
            await outcome(
                on.put("documents/readme.txt", "next", { onlyIf: { etagDoesNotMatch: "*" } }),
            ),
            await outcome(
                on.put("documents/absent.txt", "next", { onlyIf: { etagMatches: etag } }),
            ),
        ];
        expect(await conditions(client)).toEqual(await conditions(bucket));

        // page listings by limit, cursor, start key and delimiter
        expect(await listings(client)).toEqual(await listings(bucket));

        // assemble a file from a part uploaded through a resumed upload, and refuse an aborted one
        const created = await client.createMultipartUpload("media/clip.bin", {
            httpMetadata: { contentType: "video/mp4" },
            customMetadata: { take: "1" },
        });
        const resumed = client.resumeMultipartUpload("media/clip.bin", created.uploadId);
        const part = await resumed.uploadPart(1, new Uint8Array([1, 2, 3, 4]));
        const assembled = await resumed.complete([part]);
        const aborted = await client.createMultipartUpload("media/draft.bin");
        await aborted.abort();
        const parts = async (on: Bucket) =>
            outcome(
                on.resumeMultipartUpload("media/draft.bin", aborted.uploadId).uploadPart(1, "x"),
            );
        expect({
            assembled: describe(assembled),
            aborted: [await parts(client), await parts(bucket)],
        }).toEqual({
            assembled: describe(await bucket.head("media/clip.bin")),
            aborted: [
                { code: "NO_SUCH_UPLOAD", message: "multipart upload does not exist" },
                { code: "NO_SUCH_UPLOAD", message: "multipart upload does not exist" },
            ],
        });

        // delete one file and several, refusing a locked one
        const deletes = [
            await outcome(client.delete("documents/locked.txt")),
            await outcome(bucket.delete("documents/locked.txt")),
            await outcome(client.delete("documents/empty.txt")),
            await outcome(
                client.delete(["documents/readme.txt", "documents/hashed.txt", "missing"]),
            ),
        ];
        expect({ deletes, listed: await client.list() }).toEqual({
            deletes: [
                {
                    code: "ACCESS_DENIED",
                    message: "file documents/locked.txt is retained until its lock expires",
                },
                {
                    code: "ACCESS_DENIED",
                    message: "file documents/locked.txt is retained until its lock expires",
                },
                "done",
                "done",
            ],
            listed: await bucket.list(),
        });
    },
);

test("refuse a client signing with another secret, keeping the bucket unchanged", async () => {
    // serve a local bucket over S3 at a port, and sign a write under a forged secret
    await using simulation = new Simulation({ seed: 1, start: START });
    const world = simulation.environment("host");
    const served = await serveBucket(world);
    await using bucket = served.bucket;
    const location = listen(served.server, world);
    const credentials = { ...CREDENTIALS, secretAccessKey: "forged" };
    const forged = S3Client.of({ location, credentials }, world);

    // refuse the write by its signature
    expect({
        forged: await refusal(forged.put("notes/plan.txt", "forged")),
        kept: await bucket.get("notes/plan.txt"),
    }).toEqual({
        forged: [
            "FORBIDDEN",
            "SIGNATURE_DOES_NOT_MATCH",
            "the request signature does not match the signature calculated with the secret key",
        ],
        kept: null,
    });
});

test("read a file stored with a content encoding as its stored bytes, never decoded on the way", async () => {
    // store gzip bytes under a gzip content encoding in a local bucket served over S3
    await using simulation = new Simulation({ seed: 1, start: START });
    const world = simulation.environment("host");
    const served = await serveBucket(world);
    await using bucket = served.bucket;
    const encoded = await new Response(
        new Blob(["compressed notes"]).stream().pipeThrough(world.compression.compress("gzip")),
    ).bytes();
    await bucket.put("notes.txt.gz", encoded, { httpMetadata: { contentEncoding: "gzip" } });

    // read it through a signing client over the world's network
    const client = S3Client.of(
        { location: listen(served.server, world), credentials: CREDENTIALS },
        world,
    );
    const file = await client.get("notes.txt.gz");

    expect({
        encoding: file?.httpMetadata.contentEncoding,
        isStored: (await file?.bytes())?.every((byte, index) => byte === encoded[index]),
    }).toEqual({ encoding: "gzip", isStored: true });
});
