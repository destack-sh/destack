import { testSimulation } from "@destack/test/world";
import { nth, present } from "@destack/schema";
import { createHash } from "node:crypto";
import { expect, refusal, test } from "@destack/test";
import { join } from "node:path";
import { sql } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { part, upload } from "../catalogue/stack/db.ts";
import { CatalogueStorage } from "../catalogue/storage.ts";
import type { UploadedPart } from "../bucket/index.ts";
import { collectLater } from "./test/collect.ts";
import { LocalBucket } from "./index.ts";
import { Simulation } from "@destack/environment/simulation";
import { UPLOAD_RETENTION } from "../catalogue/multipart.ts";

test("resume multipart uploads after restart and retain files when completion fails", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("multipart-restart-");
    let uploadId: string;
    let selected: UploadedPart;
    {
        await using bucket = await LocalBucket.open(directory, "space-test", world);
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
        await using bucket = await LocalBucket.open(directory, "space-test", world);
        const multipart = bucket.resumeMultipartUpload("document", uploadId);
        expect(await refusal(multipart.complete([{ ...selected, etag: "changed" }]))).toEqual([
            "BAD_REQUEST",
            "PART_INVALID",
            "selected part is invalid: it is missing or has changed",
        ]);
        expect(await present(await bucket.get("document"), "document").text()).toBe("original");
        const file = present(await multipart.complete([selected]), "file");
        expect(await present(await bucket.get("document"), "document").text()).toBe("replacement");
        expect(file.customMetadata).toEqual({ author: "alice" });
        expect(file.httpMetadata).toEqual({
            contentType: "text/plain",
            cacheExpiry: new Date("2030-01-01T00:00:00Z"),
        });
        expect(await refusal(multipart.complete([selected]))).toEqual([
            "NOT_FOUND",
            "UPLOAD_NOT_FOUND",
            "multipart upload not found",
        ]);
    }
    {
        await using bucket = await LocalBucket.open(directory, "space-test", world);
        const file = await bucket.head("document");
        await collectLater(simulation, bucket);
        expect(
            (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
        ).toEqual([digestOf("replacement")]);
        const aborted = await bucket.createMultipartUpload("document");
        await aborted.uploadPart(1, "discard");
        await aborted.abort();
        expect(await bucket.head("document")).toEqual(file);
        await collectLater(simulation, bucket);
        expect(
            (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
        ).toEqual([digestOf("replacement")]);
    }
});

test("reject undersized completion and an upload part that finishes after abort", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("multipart-abort-");
    {
        await using bucket = await LocalBucket.open(directory, "space-test", world);
        const original = await bucket.put("document", "original");
        const multipart = await bucket.createMultipartUpload("document");
        const uploaded = [
            await multipart.uploadPart(1, "small"),
            await multipart.uploadPart(2, "tail"),
        ];
        expect(await refusal(multipart.complete(uploaded))).toEqual([
            "BAD_REQUEST",
            "PART_INVALID",
            "multipart part size is invalid: parts need equal sizes of at least five MiB, except the final part",
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
            "UPLOAD_NOT_FOUND",
            "multipart upload not found",
        ]);
        expect(await bucket.head("document")).toEqual(original);
    }

    // reopen the bucket, and collect the aborted upload's blobs once their leases end
    await using bucket = await LocalBucket.open(directory, "space-test", world);
    await collectLater(simulation, bucket);
    expect(
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
    ).toEqual([digestOf("original")]);
});

test("reclaim expired multipart contents once a reopened bucket sweeps", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("multipart-expire-");
    {
        await using bucket = await LocalBucket.open(directory, "space-test", world);
        await bucket.put("document", "retained");
        const multipart = await bucket.createMultipartUpload("document");
        await multipart.uploadPart(1, "expired");
    }

    // advance the stored expiration while the bucket is closed
    const database = await connectBunSqlite(
        join(directory, "bucket.db"),
        CatalogueStorage.database,
        {
            environment: world,
        },
    );
    try {
        await database.transaction(
            async (transaction) => await transaction.run(sql`UPDATE ${upload} SET expires = 0`),
        );
    } finally {
        await database.close();
    }

    await using bucket = await LocalBucket.open(directory, "space-test", world);
    const file = present(await bucket.get("document"), "document");
    expect([await file.text(), await bucket.sweep()]).toEqual(["retained", true]);
    await collectLater(simulation, bucket);
    expect(
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
    ).toEqual([digestOf("retained")]);
});

test("read a completed multipart file across part boundaries and collect its parts once deleted", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("multipart-segments-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);
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
    expect(
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name)
            .length,
    ).toBe(3);
    expect(file.size).toBe(contents.length);
    await bucket.delete("document");
    await collectLater(simulation, bucket);
    expect(
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
    ).toEqual([digestOf("kept")]);
});

test("list uploads and parts, and copy parts from files", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("multipart-list-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);
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
        "PRECONDITION_BLOCKED",
        "cannot write the file: at least one of the preconditions did not hold",
    ]);
    expect(await refusal(first.uploadPartCopy(3, "missing"))).toEqual([
        "NOT_FOUND",
        "KEY_NOT_FOUND",
        "source file 'missing' not found",
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
        "UPLOAD_NOT_FOUND",
        "multipart upload not found",
    ]);
    await first.abort();
    await second.abort();
    expect(await bucket.listUploads()).toEqual({ uploads: [], truncated: false });
});

/** Name a blob by the SHA-256 digest of its text. */
function digestOf(text: string): string {
    return createHash("sha256").update(text).digest("hex");
}

test("expire an upload at its seven days as the next upload starts or the bucket closes, reclaiming its parts, and keep it through writes", async () => {
    // start an upload with a part
    await using simulation = new Simulation({ seed: 1, start: Date.parse("2026-10-01T00:00:00Z") });
    const { clock } = simulation;
    const world = simulation.environment("bucket");
    const directory = await world.files.makeTemporaryDirectory("multipart-expiry-");
    const bucket = await LocalBucket.open(directory, "space-test", world);
    const multipart = await bucket.createMultipartUpload("document");
    await multipart.uploadPart(1, "part");

    // keep it through a sweep before its seven days, and through a write once they pass
    await simulation.jump(clock.now() + UPLOAD_RETENTION - 1);
    await bucket.sweep();
    const before = await uploadsOf(bucket);
    await simulation.jump(clock.now() + 1);
    await bucket.put("other", "kept");
    const written = await uploadsOf(bucket);

    // expire it and its part as the next upload starts, and that one with its part on the close
    const draft = await bucket.createMultipartUpload("draft");
    const started = await uploadsOf(bucket);
    await draft.uploadPart(1, "draft part");
    await simulation.jump(clock.now() + UPLOAD_RETENTION);
    await bucket[Symbol.asyncDispose]();
    await using reopened = await LocalBucket.open(directory, "space-test", world);
    expect({ before, written, started, closed: await uploadsOf(reopened) }).toEqual({
        before: [1, 1],
        written: [1, 1],
        started: [1, 0],
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
