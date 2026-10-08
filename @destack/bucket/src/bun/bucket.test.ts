import { present } from "@destack/schema";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { expect, refusal, test } from "@destack/test";
import { mkdir, mkdtemp, readdir, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type UploadedPart, StorageClass } from "../bucket/index.ts";
import { LocalBucket } from "./index.ts";

test("retain local files across reopen and failed streamed uploads", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-reopen-"));
    try {
        // refuse creating a bucket without the scope it belongs to
        await expect(LocalBucket.open(directory)).rejects.toMatchObject({
            code: "NO_SUCH_BUCKET",
            message: `no bucket at ${directory}`,
        });

        {
            await using bucket = await LocalBucket.open(directory, "space-test");
            await bucket.put("document", "original", {
                httpMetadata: { contentType: "text/plain" },
            });
            const failure = new Error("upload interrupted");
            const body = new ReadableStream<Uint8Array>({
                start(controller) {
                    controller.enqueue(new TextEncoder().encode("partial"));
                    controller.error(failure);
                },
            });
            await expect(bucket.put("document", body)).rejects.toThrow(failure);
            await expect(
                bucket.put("document", "changed", { sha256: "0".repeat(64) }),
            ).rejects.toMatchObject({
                code: "INVALID_CHECKSUM",
                message: "object checksum does not match its contents",
            });
        }
        await using restored = await LocalBucket.open(directory, "space-test");
        const document = present(await restored.get("document"), "document");
        expect(await new Response(document.body).text()).toBe("original");
        expect(document.httpMetadata.contentType).toBe("text/plain");
        expect(await readdir(join(directory, "files"))).toEqual([digestOf("original")]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("recover the catalogue after terminating its host, and sweep its interrupted upload away", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-crash-"));
    const module = new URL("./index.ts", import.meta.url).href;
    const child = Bun.spawn(
        [
            process.execPath,
            "--no-env-file",
            "--preload",
            fileURLToPath(import.meta.resolve("@destack/package/bun/preload")),
            "--eval",
            `
            import { LocalBucket } from ${JSON.stringify(module)};
            setInterval(() => {}, 1000);
            const bucket = await LocalBucket.open(${JSON.stringify(directory)}, "space-test");
            await bucket.put("document", "committed");
            let sent = false;
            await bucket.put("document", new ReadableStream({
                pull(controller) {
                    if (!sent) {
                        sent = true;
                        controller.enqueue(new TextEncoder().encode("partial"));
                    } else {
                        console.log("ready");
                        return new Promise(() => {});
                    }
                }
            }));
        `,
        ],
        {
            stdout: "pipe",
            stderr: "inherit",
        },
    );
    let isTerminated = false;
    try {
        const reader = child.stdout.getReader();
        const ready = await reader.read();
        reader.releaseLock();
        expect(new TextDecoder().decode(ready.value)).toBe("ready\n");
        child.kill("SIGKILL");
        await child.exited;
        isTerminated = true;
        expect(child.signalCode).toBe("SIGKILL");
        await child.stdout.cancel();

        // reopen the bucket once the operating system releases the lock, keeping the interrupted write until a sweep
        await using restored = await LocalBucket.open(directory, "space-test");
        const document = present(await restored.get("document"), "document");
        const reopened = (await readdir(join(directory, "files"))).length;
        expect([
            await new Response(document.body).text(),
            reopened,
            await restored.sweep(),
            await readdir(join(directory, "files")),
        ]).toEqual(["committed", 2, true, [digestOf("committed")]]);
    } finally {
        if (!isTerminated) {
            child.kill("SIGKILL");
            await child.exited;
        }
        await rm(directory, { recursive: true });
    }
});

test("close storage immediately after consuming or cancelling a body", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-close-"));
    try {
        for (const operation of ["consume", "cancel"] as const) {
            await using bucket = await LocalBucket.open(directory, "space-test");
            await bucket.put("document", new Uint8Array(128 * 1024));
            const file = present(await bucket.get("document"), "file");
            if (operation === "consume") {
                expect((await file.bytes()).length).toBe(128 * 1024);
            } else {
                await file.body.cancel();
            }
        }
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("sweep multiple batches of abandoned files without deleting files or upload parts, leaving them at open", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-recovery-"));
    try {
        let uploadId: string;
        let selected: UploadedPart;
        {
            await using bucket = await LocalBucket.open(directory, "space-test");
            await bucket.put("document", "retained");
            const upload = await bucket.createMultipartUpload("multipart");
            uploadId = upload.uploadId;
            selected = await upload.uploadPart(1, "retained part");
        }

        // leave more unreferenced blobs than one sweep query can take, and an interrupted write
        for (let index = 0; index < 501; index++) {
            await writeFile(join(directory, "files", digestOf(`abandoned ${index}`)), "abandoned");
        }
        await writeFile(join(directory, "files", `.${crypto.randomUUID()}`), "interrupted");
        await using bucket = await LocalBucket.open(directory, "space-test");
        const reopened = (await readdir(join(directory, "files"))).length;
        expect([
            await present(await bucket.get("document"), "document").text(),
            reopened,
            await bucket.sweep(),
            (await readdir(join(directory, "files"))).length,
        ]).toEqual(["retained", 504, true, 2]);
        await bucket.resumeMultipartUpload("multipart", uploadId).complete([selected]);
        expect(await present(await bucket.get("multipart"), "multipart").text()).toBe(
            "retained part",
        );
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("report reclamation failures separately from committed file replacements", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-collect-"));
    try {
        await using bucket = await LocalBucket.open(directory, "space-test");
        await bucket.put("document", "original");
        const path = join(directory, "files", digestOf("original"));
        const retained = `${path}.retained`;

        // prevent unlinking the old version without changing the published replacement
        await rename(path, retained);
        await mkdir(path);
        await writeFile(join(path, "blocked"), "prevent removal");
        try {
            const replacement = await bucket.put("document", "replacement");
            expect(await bucket.head("document")).toEqual(replacement);
            await expect(bucket.collect()).rejects.toMatchObject({ syscall: "unlink", path });
            await expect(bucket.put("document", "uncommitted")).rejects.toMatchObject({
                syscall: "unlink",
                path,
            });
            expect(await present(await bucket.get("document"), "document").text()).toBe(
                "replacement",
            );
        } finally {
            await rm(path, { recursive: true });
            await rename(retained, path);
        }

        await bucket.collect();
        expect(await readdir(join(directory, "files"))).toEqual([digestOf("replacement")]);
        await Promise.all([bucket[Symbol.asyncDispose](), bucket[Symbol.asyncDispose]()]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("copy files by sharing their contents until the last reference goes", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-copy-"));
    try {
        await using bucket = await LocalBucket.open(directory, "space-test");
        const source = await bucket.put("source", "shared", {
            httpMetadata: { contentType: "text/plain" },
            customMetadata: { author: "alice" },
        });

        // copy with the source's metadata, or with replaced metadata and storage class
        const copy = await bucket.copy("source", "copy");
        const replaced = await bucket.copy("source", "replaced", {
            httpMetadata: { contentType: "text/markdown" },
            customMetadata: {},
            storageClass: "InfrequentAccess",
        });
        expect([copy, replaced]).toEqual([
            await bucket.head("copy"),
            await bucket.head("replaced"),
        ]);
        expect({
            etags: [copy.etag, replaced.etag],
            isVersionNew: copy.version !== source.version,
            httpMetadata: [copy.httpMetadata, replaced.httpMetadata],
            customMetadata: [copy.customMetadata, replaced.customMetadata],
            storageClass: [copy.storageClass, replaced.storageClass],
        }).toEqual({
            etags: [source.etag, source.etag],
            isVersionNew: true,
            httpMetadata: [{ contentType: "text/plain" }, { contentType: "text/markdown" }],
            customMetadata: [{ author: "alice" }, {}],
            storageClass: ["Standard", "InfrequentAccess"],
        });

        // refuse a missing source, and return null when a source precondition fails
        await expect(bucket.copy("missing", "copy")).rejects.toMatchObject({
            code: "NO_SUCH_KEY",
            message: "the source file does not exist",
        });
        await expect(
            bucket.copy("source", "copy", { onlyIf: { etagMatches: "other" } }),
        ).rejects.toMatchObject({ code: "PRECONDITION_FAILED" });

        // keep the one blob, named by its digest, while any copy references it
        await bucket.delete(["source", "copy"]);
        await bucket.collect();
        expect(await readdir(join(directory, "files"))).toEqual([digestOf("shared")]);
        expect(await present(await bucket.get("replaced"), "replaced").text()).toBe("shared");
        await bucket.delete("replaced");
        await bucket.collect();
        expect(await readdir(join(directory, "files"))).toEqual([]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("record storage classes and refuse storage classes R2 does not offer", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-class-"));
    try {
        await using bucket = await LocalBucket.open(directory, "space-test");
        const cold = await bucket.put("cold", "rarely read", { storageClass: "InfrequentAccess" });
        expect({
            put: cold.storageClass,
            head: present(await bucket.head("cold"), "cold").storageClass,
        }).toEqual({
            put: "InfrequentAccess",
            head: "InfrequentAccess",
        });
        const listed = await bucket.list();
        expect(listed.files.map((file) => file.storageClass)).toEqual(["InfrequentAccess"]);

        // refuse storage classes R2 does not offer when reading them from a caller
        await expect(Promise.try(() => StorageClass.read("Glacier"))).rejects.toMatchObject({
            code: "INVALID_STORAGE_CLASS",
            message: "unknown storage class Glacier",
        });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("encrypt files under customer keys, reading ranges with the key and refusing any other", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-customer-"));
    try {
        await using bucket = await LocalBucket.open(directory, "space-test");
        const ssecKey = "a1".repeat(32);
        const md5 = createHash("md5").update(Uint8Array.fromHex(ssecKey)).digest("base64");
        const text = "customer encrypted contents, longer than one AES block";

        // store the file encrypted, and keep no plaintext on disk
        const stored = await bucket.put("sealed", text, { ssecKey });
        const onDisk = await Promise.all(
            (await readdir(join(directory, "files"))).map((name) =>
                Bun.file(join(directory, "files", name)).text(),
            ),
        );
        expect([
            stored.ssecKeyMd5,
            present(await bucket.head("sealed"), "sealed").ssecKeyMd5,
            onDisk.includes(text),
        ]).toEqual([md5, md5, false]);

        // read the whole file and an unaligned range with the key
        const whole = present(await bucket.get("sealed", { ssecKey }), "whole");
        const range = present(
            await bucket.get("sealed", { ssecKey, range: { offset: 21, length: 13 } }),
            "range",
        );
        expect([await whole.text(), await range.text()]).toEqual([text, text.slice(21, 34)]);

        // refuse reading without the key, with another key, and a plain file with a key
        await bucket.put("plain", "open");
        const refusals = await Promise.all(
            [
                bucket.get("sealed"),
                bucket.get("sealed", { ssecKey: "b2".repeat(32) }),
                bucket.get("plain", { ssecKey }),
                bucket.put("short", "x", { ssecKey: "00" }),
            ].map((pending) => refusal(pending)),
        );
        expect(refusals).toEqual([
            ["INVALID_CUSTOMER_KEY", "the file is encrypted with another customer key"],
            ["INVALID_CUSTOMER_KEY", "the file is encrypted with another customer key"],
            ["INVALID_CUSTOMER_KEY", "the file is not encrypted with a customer key"],
            ["INVALID_CUSTOMER_KEY", "a customer key is 32 bytes or 64 hexadecimal digits"],
        ]);

        // upload parts under the upload's key, refusing a part without it
        const upload = await bucket.createMultipartUpload("parts", { ssecKey });
        await expect(upload.uploadPart(1, "first")).rejects.toMatchObject({
            code: "INVALID_CUSTOMER_KEY",
            message: "the file is encrypted with another customer key",
        });
        const part = await upload.uploadPart(1, "parted contents", { ssecKey });
        const completed = await upload.complete([part]);
        expect([
            completed.ssecKeyMd5,
            await present(await bucket.get("parts", { ssecKey }), "parts").text(),
        ]).toEqual([md5, "parted contents"]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

/** Name a blob by the SHA-256 digest of its text. */
function digestOf(text: string): string {
    return createHash("sha256").update(text).digest("hex");
}

test("count the bytes a bucket's published files keep, a replaced file once and a deleted one not at all", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-bytes-"));
    try {
        // publish two files, replace one and delete the other
        await using bucket = await LocalBucket.open(directory, "space-test");
        const empty = await bucket.bytes();
        await bucket.put("plans", "twelve bytes");
        await bucket.put("notes", "five!");
        const both = await bucket.bytes();
        await bucket.put("plans", "four");
        await bucket.delete("notes");

        // count each published file's current size
        expect({ empty, both, left: await bucket.bytes() }).toEqual({
            empty: 0,
            both: 17,
            left: 4,
        });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("refuse replacing or deleting a locked file until its retain-until date, then allow both", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-lock-"));
    try {
        // lock a file for a moment beside an unlocked one
        await using bucket = await LocalBucket.open(directory, "space-test");
        const retainUntil = new Date(Date.now() + 200);
        await bucket.put("audit/segment", "sealed", { retainUntil });
        await bucket.put("notes", "free");

        // refuse replacing it, and refuse a batch deleting it, keeping the batch's other file
        const replaced = await refusal(bucket.put("audit/segment", "changed"));
        const deleted = await refusal(bucket.delete(["notes", "audit/segment"]));
        const held = [
            (await bucket.head("audit/segment"))?.retainUntil?.getTime(),
            (await bucket.head("notes")) === null,
        ];

        // once the date passes, delete it
        await new Promise((resolve) => {
            setTimeout(resolve, 250);
        });
        await bucket.delete("audit/segment");
        expect([replaced, deleted, held, await bucket.head("audit/segment")]).toEqual([
            ["LOCKED", "file audit/segment is retained until its lock expires"],
            ["LOCKED", "file audit/segment is retained until its lock expires"],
            [retainUntil.getTime(), false],
            null,
        ]);
    } finally {
        await rm(directory, { recursive: true });
    }
});
