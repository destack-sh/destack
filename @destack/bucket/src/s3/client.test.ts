import { createHash } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import type { Bucket, BucketFile } from "../bucket/index.ts";
import { BucketError, BucketPreconditionError } from "../error/index.ts";
import { LocalBucket } from "../bun/index.ts";
import { S3Client } from "./client.ts";
import { S3Server } from "./server.ts";

/** The access key the host authorized the client's requests for. */
const ACCESS_KEY_ID = "space-test";

/** A customer key encrypting a file, as 64 hexadecimal digits. */
const CUSTOMER_KEY = "07".repeat(32);

/** Describe a file as plain values, its upload time in the whole seconds HTTP carries. */
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

/** Run an operation and describe its refusal. */
async function refusal(operation: Promise<unknown>) {
    try {
        await operation;

        return "done";
    } catch (error) {
        if (error instanceof BucketPreconditionError) {
            return { code: error.code, current: describe(error.current) };
        } else if (error instanceof BucketError) {
            return { code: error.code, message: error.message };
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

test("roundtrip every bucket operation through S3, answering as the bucket answers directly", async () => {
    // serve a local bucket over S3 to a client the host already authorized
    const directory = await mkdtemp(join(tmpdir(), "destack-s3-client-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    await using bucket = await LocalBucket.open(join(directory, "files"), ACCESS_KEY_ID);
    const server = new S3Server({
        region: "auto",
        credentials: async () => undefined,
        open: async (name, accessKeyId) =>
            name === "files" && accessKeyId === ACCESS_KEY_ID ? bucket : undefined,
    });
    const client = new S3Client("https://s3.test/files/", (request) =>
        server.serve(request, ACCESS_KEY_ID),
    );

    // store files with metadata, a storage class, a checksum and a customer key, as the bucket then holds them
    const body = "Hello, world!";
    const expiry = new Date(Date.UTC(2027, 0, 1));
    const stored = await client.put("documents/readme.txt", body, {
        httpMetadata: { contentType: "text/plain", cacheControl: "no-cache", cacheExpiry: expiry },
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
        retainUntil: new Date(Date.now() + 3_600_000),
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
        await refusal(on.get("documents/sealed.txt")),
        await refusal(on.get("documents/readme.txt", { range: { offset: 100 } })),
    ];
    expect(await reads(client)).toEqual(await reads(bucket));

    // refuse failed preconditions of reads and writes with the file they met, and pass held ones
    const etag = stored.etag;
    const second = new Date(Math.floor(Date.now() / 1000) * 1000 - 60_000);
    const conditions = async (on: Bucket) => [
        await refusal(on.get("documents/readme.txt", { onlyIf: { etagMatches: "other" } })),
        await refusal(on.get("documents/readme.txt", { onlyIf: { etagDoesNotMatch: etag } })),
        await refusal(on.get("documents/readme.txt", { onlyIf: { uploadedBefore: second } })),
        await read(on, "documents/readme.txt", { onlyIf: { etagMatches: etag } }),
        await read(on, "documents/readme.txt", { onlyIf: { uploadedAfter: second } }),
        await refusal(
            on.put("documents/readme.txt", "next", { onlyIf: { etagDoesNotMatch: "*" } }),
        ),
        await refusal(on.put("documents/absent.txt", "next", { onlyIf: { etagMatches: etag } })),
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
        refusal(on.resumeMultipartUpload("media/draft.bin", aborted.uploadId).uploadPart(1, "x"));
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
        await refusal(client.delete("documents/locked.txt")),
        await refusal(bucket.delete("documents/locked.txt")),
        await refusal(client.delete("documents/empty.txt")),
        await refusal(client.delete(["documents/readme.txt", "documents/hashed.txt", "missing"])),
    ];
    expect({ deletes, listed: await client.list() }).toEqual({
        deletes: [
            {
                code: "LOCKED",
                message: "file documents/locked.txt is retained until its lock expires",
            },
            {
                code: "LOCKED",
                message: "file documents/locked.txt is retained until its lock expires",
            },
            "done",
            "done",
        ],
        listed: await bucket.list(),
    });
});
