import { nth, present } from "@destack/schema";
import { Clock } from "@destack/environment";
import { createHash } from "node:crypto";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { sql } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { catalogueDatabase, part, upload } from "../catalogue/stack/db.ts";
import type { UploadedPart } from "../bucket/index.ts";
import { LocalBucket } from "./index.ts";
import { VirtualClock } from "@destack/environment/simulation";
import { UPLOAD_RETENTION } from "../catalogue/multipart.ts";

test("resume multipart uploads after restart and retain files when completion fails", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-restart-"));
    let uploadId: string;
    let selected: UploadedPart;
    try {
        {
            await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
            await bucket.put("document", "original");
            const multipart = await bucket.createMultipartUpload("document", {
                httpMetadata: {
                    contentType: "text/plain",
                    cacheExpiry: new Date("2030-01-01T00:00:00Z"),
                },
                customMetadata: { author: "alice" },
            });
            uploadId = multipart.uploadId;
            selected = await multipart.uploadPart(1, "replacement");
        }
        {
            await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
            const multipart = bucket.resumeMultipartUpload("document", uploadId);
            expect(await refusal(multipart.complete([{ ...selected, etag: "changed" }]))).toEqual([
                "BAD_REQUEST",
                "INVALID_PART",
                "a selected part is missing or has changed",
            ]);
            expect(await present(await bucket.get("document"), "document").text()).toBe("original");
            const file = present(await multipart.complete([selected]), "file");
            expect(await present(await bucket.get("document"), "document").text()).toBe(
                "replacement",
            );
            expect(file.customMetadata).toEqual({ author: "alice" });
            expect(file.httpMetadata).toEqual({
                contentType: "text/plain",
                cacheExpiry: new Date("2030-01-01T00:00:00Z"),
            });
            expect(await refusal(multipart.complete([selected]))).toEqual([
                "NOT_FOUND",
                "NO_SUCH_UPLOAD",
                "multipart upload does not exist",
            ]);
        }
        {
            await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
            const file = await bucket.head("document");
            expect(await readdir(join(directory, "files"))).toEqual([digestOf("replacement")]);
            const aborted = await bucket.createMultipartUpload("document");
            await aborted.uploadPart(1, "discard");
            await aborted.abort();
            expect(await bucket.head("document")).toEqual(file);
            await bucket.collect();
            expect(await readdir(join(directory, "files"))).toEqual([digestOf("replacement")]);
        }
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reject undersized completion and an upload part that finishes after abort", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-abort-"));
    try {
        {
            await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
            const original = await bucket.put("document", "original");
            const multipart = await bucket.createMultipartUpload("document");
            const uploaded = [
                await multipart.uploadPart(1, "small"),
                await multipart.uploadPart(2, "tail"),
            ];
            expect(await refusal(multipart.complete(uploaded))).toEqual([
                "BAD_REQUEST",
                "INVALID_PART",
                "multipart parts require equal sizes of at least five MiB, except the final part",
            ]);
            expect(await bucket.head("document")).toEqual(original);

            // finish writing bytes only after another caller has discarded the upload
            const started = Promise.withResolvers<void>();
            const finish = Promise.withResolvers<void>();
            const writing = multipart.uploadPart(
                3,
                new ReadableStream<Uint8Array>({
                    async start(controller) {
                        controller.enqueue(new TextEncoder().encode("partial"));
                        started.resolve();
                        await finish.promise;
                        controller.close();
                    },
                }),
            );
            await started.promise;
            await multipart.abort();
            finish.resolve();
            expect(await refusal(writing)).toEqual([
                "NOT_FOUND",
                "NO_SUCH_UPLOAD",
                "multipart upload does not exist",
            ]);
            expect(await bucket.head("document")).toEqual(original);
        }

        // reopen the bucket to sweep the aborted upload's blobs
        await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
        expect(await readdir(join(directory, "files"))).toEqual([digestOf("original")]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reclaim expired multipart contents once a reopened bucket sweeps", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-expire-"));
    try {
        {
            await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
            await bucket.put("document", "retained");
            const multipart = await bucket.createMultipartUpload("document");
            await multipart.uploadPart(1, "expired");
        }

        // advance the stored expiration while the bucket is closed
        const database = await connectBunSqlite(join(directory, "bucket.db"), catalogueDatabase);
        try {
            await database.run(sql`UPDATE ${upload} SET expires = 0`);
        } finally {
            await database.close();
        }

        await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
        const file = present(await bucket.get("document"), "document");
        expect([await file.text(), await bucket.sweep()]).toEqual(["retained", true]);
        expect(await readdir(join(directory, "files"))).toEqual([digestOf("retained")]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("read a completed multipart file across part boundaries and collect its parts once deleted", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-segments-"));
    try {
        await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
        await bucket.put("retained", "kept");
        const multipart = await bucket.createMultipartUpload("document");

        // publish a full-size first part and a short final part without copying their bytes
        const first = new Uint8Array(5 * 1024 * 1024).map((_, index) => index % 251);
        const last = new TextEncoder().encode("final bytes");
        const file = await multipart.complete([
            await multipart.uploadPart(1, first),
            await multipart.uploadPart(2, last),
        ]);
        const contents = new Uint8Array(first.length + last.length);
        contents.set(first);
        contents.set(last, first.length);
        // compare the five MiB directly, since a structural comparison walks every byte slowly
        const restored = await present(await bucket.get("document"), "document").bytes();
        expect(Buffer.compare(restored, contents)).toBe(0);

        // read a range that ends three bytes into the final part
        const offset = first.length - 3;
        const ranged = present(
            await bucket.get("document", { range: { offset, length: 6 } }),
            "ranged",
        );
        expect(await ranged.bytes()).toEqual(contents.subarray(offset, offset + 6));

        // keep both part contents until the file is deleted, then collect them
        expect((await readdir(join(directory, "files"))).length).toBe(3);
        expect(file.size).toBe(contents.length);
        await bucket.delete("document");
        await bucket.collect();
        expect(await readdir(join(directory, "files"))).toEqual([digestOf("kept")]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("list uploads and parts, and copy parts from files", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-list-"));
    try {
        await using bucket = await LocalBucket.open(directory, "space-test", Clock.system);
        await bucket.put("source", "0123456789");
        const first = await bucket.createMultipartUpload("media/a", {
            storageClass: "InfrequentAccess",
        });
        const second = await bucket.createMultipartUpload("media/b");
        const other = await bucket.createMultipartUpload("other");

        // list uploads by prefix and continue after a key marker
        const uploads = await bucket.listUploads({ prefix: "media/", limit: 1 });
        expect(uploads).toEqual({
            uploads: [
                {
                    key: "media/a",
                    uploadId: first.uploadId,
                    initiated: nth(uploads.uploads, 0).initiated,
                    storageClass: "InfrequentAccess",
                },
            ],
            truncated: true,
        });
        const next = await bucket.listUploads({ prefix: "media/", keyMarker: "media/a" });
        expect(next.uploads.map((entry) => entry.uploadId)).toEqual([second.uploadId]);
        expect(next.truncated).toBe(false);

        // copy a whole file and a range of it into parts, and list them in order
        const whole = await first.uploadPartCopy(2, "source");
        const range = await first.uploadPartCopy(1, "source", { range: { offset: 2, length: 3 } });
        expect(
            await refusal(first.uploadPartCopy(3, "source", { onlyIf: { etagMatches: "other" } })),
        ).toEqual([
            "PRECONDITION_FAILED",
            "PRECONDITION_UNMET",
            "at least one of the preconditions did not hold",
        ]);
        expect(await refusal(first.uploadPartCopy(3, "missing"))).toEqual([
            "NOT_FOUND",
            "NO_SUCH_KEY",
            "the source file does not exist",
        ]);
        const parts = await first.listParts();
        expect(parts).toEqual({
            parts: [
                { ...range, size: 3, uploaded: nth(parts.parts, 0).uploaded },
                { ...whole, size: 10, uploaded: nth(parts.parts, 1).uploaded },
            ],
            truncated: false,
        });
        const page = await first.listParts({ partNumberMarker: 1, limit: 1 });
        expect(page.parts.map((entry) => entry.partNumber)).toEqual([2]);

        // refuse listing the parts of an aborted upload
        await other.abort();
        expect(await refusal(other.listParts())).toEqual([
            "NOT_FOUND",
            "NO_SUCH_UPLOAD",
            "multipart upload does not exist",
        ]);
        await first.abort();
        await second.abort();
        expect(await bucket.listUploads()).toEqual({ uploads: [], truncated: false });
    } finally {
        await rm(directory, { recursive: true });
    }
});

/** Name a blob by the SHA-256 digest of its text. */
function digestOf(text: string): string {
    return createHash("sha256").update(text).digest("hex");
}

test("expire an upload at its seven days on a sweep or a close, reclaiming its parts, and keep it through writes", async () => {
    // start an upload with a part
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-expiry-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const clock = new VirtualClock(Date.parse("2026-10-01T00:00:00Z"));
    const bucket = await LocalBucket.open(directory, "space-test", clock);
    let isClosed = false;
    onTestFinished(async () => {
        if (!isClosed) {
            await bucket[Symbol.asyncDispose]();
        }
    });
    const multipart = await bucket.createMultipartUpload("document");
    await multipart.uploadPart(1, "part");

    // keep it through a sweep before its seven days, and through a write once they pass
    clock.advance(UPLOAD_RETENTION - 1);
    await bucket.sweep();
    const before = await uploadsOf(bucket);
    clock.advance(1);
    await bucket.put("other", "kept");
    const written = await uploadsOf(bucket);

    // expire it and its part on the sweep, and another one with its part on the close
    await bucket.sweep();
    const swept = await uploadsOf(bucket);
    const draft = await bucket.createMultipartUpload("draft");
    await draft.uploadPart(1, "draft part");
    clock.advance(UPLOAD_RETENTION);
    isClosed = true;
    await bucket[Symbol.asyncDispose]();
    const reopened = await LocalBucket.open(directory, "space-test", clock);
    onTestFinished(() => reopened[Symbol.asyncDispose]());
    expect({ before, written, swept, closed: await uploadsOf(reopened) }).toEqual({
        before: [1, 1],
        written: [1, 1],
        swept: [0, 0],
        closed: [0, 0],
    });
});

/** Count a bucket's uploads and their parts. */
async function uploadsOf(bucket: LocalBucket): Promise<[number, number]> {
    const { database } = bucket.database();

    return [
        (await database.select({ id: upload.id }).from(upload)).length,
        (await database.select({ blob: part.blob }).from(part)).length,
    ];
}
