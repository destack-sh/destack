import { expect, test } from "@destack/test";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { sql } from "@destack/db";
import { connect } from "@destack/db/turso";
import { catalogueDatabase, upload } from "./stack/db.ts";
import type { UploadedPart } from "../bucket/index.ts";
import { LocalBucket } from "./index.ts";

test("resume multipart uploads after restart and retain files when completion fails", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-restart-"));
    let uploadId: string;
    let selected: UploadedPart;
    try {
        {
            await using bucket = await LocalBucket.open(directory);
            await bucket.put("document", "original");
            const upload = await bucket.createMultipartUpload("document", {
                httpMetadata: {
                    contentType: "text/plain",
                    cacheExpiry: new Date("2030-01-01T00:00:00Z"),
                },
                customMetadata: { author: "alice" },
            });
            uploadId = upload.uploadId;
            selected = await upload.uploadPart(1, "replacement");
        }
        {
            await using bucket = await LocalBucket.open(directory);
            const upload = bucket.resumeMultipartUpload("document", uploadId);
            await expect(upload.complete([{ ...selected, etag: "changed" }])).rejects.toMatchObject(
                {
                    code: "INVALID_PART",
                    message: "a selected part is missing or has changed",
                },
            );
            expect(await (await bucket.get("document"))!.text()).toBe("original");
            const file = await upload.complete([selected]);
            expect(await (await bucket.get("document"))!.text()).toBe("replacement");
            expect(file.customMetadata).toEqual({ author: "alice" });
            expect(file.httpMetadata).toEqual({
                contentType: "text/plain",
                cacheExpiry: new Date("2030-01-01T00:00:00Z"),
            });
            await expect(upload.complete([selected])).rejects.toMatchObject({
                code: "NO_SUCH_UPLOAD",
                message: "multipart upload does not exist",
            });
        }
        {
            await using bucket = await LocalBucket.open(directory);
            const file = await bucket.head("document");
            expect(await readdir(join(directory, "files"))).toEqual([file!.version]);
            const aborted = await bucket.createMultipartUpload("document");
            await aborted.uploadPart(1, "discard");
            await aborted.abort();
            expect(await bucket.head("document")).toEqual(file);
            await bucket.collect();
            expect(await readdir(join(directory, "files"))).toEqual([file!.version]);
        }
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reject undersized completion and an upload part that finishes after abort", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-abort-"));
    try {
        {
            await using bucket = await LocalBucket.open(directory);
            const original = await bucket.put("document", "original");
            const upload = await bucket.createMultipartUpload("document");
            const uploaded = [
                await upload.uploadPart(1, "small"),
                await upload.uploadPart(2, "tail"),
            ];
            await expect(upload.complete(uploaded)).rejects.toMatchObject({
                code: "INVALID_PART",
                message:
                    "multipart parts require equal sizes of at least five MiB, except the final part",
            });
            expect(await bucket.head("document")).toEqual(original);

            // finish writing bytes only after another caller has discarded the upload
            const started = Promise.withResolvers<void>();
            const finish = Promise.withResolvers<void>();
            const writing = upload.uploadPart(
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
            await upload.abort();
            finish.resolve();
            await expect(writing).rejects.toMatchObject({
                code: "NO_SUCH_UPLOAD",
                message: "multipart upload does not exist",
            });
            expect(await bucket.head("document")).toEqual(original);
        }
        await using bucket = await LocalBucket.open(directory);
        const file = await bucket.head("document");
        expect(await readdir(join(directory, "files"))).toEqual([file!.version]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reclaim expired multipart contents when reopening a bucket", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-expire-"));
    try {
        {
            await using bucket = await LocalBucket.open(directory);
            await bucket.put("document", "retained");
            const upload = await bucket.createMultipartUpload("document");
            await upload.uploadPart(1, "expired");
        }

        // advance the stored expiration while the bucket is closed
        const database = await connect(join(directory, "bucket.db"), catalogueDatabase);
        try {
            await database.run(sql`UPDATE ${upload} SET expires = 0`);
        } finally {
            await database.close();
        }

        await using bucket = await LocalBucket.open(directory);
        const file = await bucket.get("document");
        expect(await file!.text()).toBe("retained");
        expect(await readdir(join(directory, "files"))).toEqual([file!.version]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("read a completed multipart file across part boundaries and collect its parts once deleted", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-segments-"));
    try {
        await using bucket = await LocalBucket.open(directory);
        const retained = await bucket.put("retained", "kept");
        const upload = await bucket.createMultipartUpload("document");

        // publish a full-size first part and a short final part without copying their bytes
        const first = new Uint8Array(5 * 1024 * 1024).map((_, index) => index % 251);
        const last = new TextEncoder().encode("final bytes");
        const file = await upload.complete([
            await upload.uploadPart(1, first),
            await upload.uploadPart(2, last),
        ]);
        const contents = new Uint8Array(first.length + last.length);
        contents.set(first);
        contents.set(last, first.length);
        // compare the five MiB directly, since a structural comparison walks every byte slowly
        const restored = await (await bucket.get("document"))!.bytes();
        expect(Buffer.compare(restored, contents)).toBe(0);

        // read a range that ends three bytes into the final part
        const offset = first.length - 3;
        const ranged = await bucket.get("document", { range: { offset, length: 6 } });
        expect(await ranged!.bytes()).toEqual(contents.subarray(offset, offset + 6));

        // keep both part contents until the file is deleted, then collect them
        expect((await readdir(join(directory, "files"))).length).toBe(3);
        expect(file.size).toBe(contents.length);
        await bucket.delete("document");
        await bucket.collect();
        expect(await readdir(join(directory, "files"))).toEqual([retained.version]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("list uploads and parts, and copy parts from files", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-list-"));
    try {
        await using bucket = await LocalBucket.open(directory);
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
                    initiated: uploads.uploads[0]!.initiated,
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
        expect(await first.uploadPartCopy(3, "source", { onlyIf: { etagMatches: "other" } })).toBe(
            null,
        );
        await expect(first.uploadPartCopy(3, "missing")).rejects.toMatchObject({
            code: "NO_SUCH_KEY",
        });
        const parts = await first.listParts();
        expect(parts).toEqual({
            parts: [
                { ...range, size: 3, uploaded: parts.parts[0]!.uploaded },
                { ...whole, size: 10, uploaded: parts.parts[1]!.uploaded },
            ],
            truncated: false,
        });
        const page = await first.listParts({ partNumberMarker: 1, limit: 1 });
        expect(page.parts.map((entry) => entry.partNumber)).toEqual([2]);

        // refuse listing the parts of an aborted upload
        await other.abort();
        await expect(other.listParts()).rejects.toMatchObject({ code: "NO_SUCH_UPLOAD" });
        await first.abort();
        await second.abort();
        expect(await bucket.listUploads()).toEqual({ uploads: [], truncated: false });
    } finally {
        await rm(directory, { recursive: true });
    }
});
