import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
import { mkdir, mkdtemp, readdir, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { UploadedPart } from "../bucket/index.ts";
import { LocalBucket } from "./index.ts";

test("retain local files across reopen and failed streamed uploads", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-reopen-"));
    try {
        {
            await using bucket = await LocalBucket.open(directory);
            await bucket.put("document", "original", {
                httpMetadata: { contentType: "text/plain" },
            });
            const failure = new Error("Upload interrupted");
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
        await using restored = await LocalBucket.open(directory);
        const document = await restored.get("document");
        expect(await new Response(document!.body).text()).toBe("original");
        expect(document!.httpMetadata.contentType).toBe("text/plain");
        expect(await readdir(join(directory, "files"))).toEqual([document!.version]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("recover the catalogue and reclaim an upload after terminating its host", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-crash-"));
    const module = new URL("./index.ts", import.meta.url).href;
    const child = Bun.spawn(
        [
            process.execPath,
            "--no-env-file",
            "--preload",
            fileURLToPath(import.meta.resolve("@destack/package/transform/preload")),
            "--eval",
            `
            import { LocalBucket } from ${JSON.stringify(module)};
            setInterval(() => {}, 1000);
            const bucket = await LocalBucket.open(${JSON.stringify(directory)});
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

        // reopen the bucket once the operating system releases the lock, removing uncommitted contents
        await using restored = await LocalBucket.open(directory);
        const document = await restored.get("document");
        expect(await new Response(document!.body).text()).toBe("committed");
        expect(await readdir(join(directory, "files"))).toEqual([document!.version]);
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
            await using bucket = await LocalBucket.open(directory);
            await bucket.put("document", new Uint8Array(128 * 1024));
            const file = await bucket.get("document");
            if (operation === "consume") {
                expect((await file!.bytes()).length).toBe(128 * 1024);
            } else {
                await file!.body.cancel();
            }
        }
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("recover multiple batches of abandoned files without deleting files or upload parts", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-recovery-"));
    try {
        let uploadId: string;
        let selected: UploadedPart;
        {
            await using bucket = await LocalBucket.open(directory);
            await bucket.put("document", "retained");
            const upload = await bucket.createMultipartUpload("multipart");
            uploadId = upload.uploadId;
            selected = await upload.uploadPart(1, "retained part");
        }

        // leave more unreferenced files than one recovery query can take
        for (let index = 0; index < 501; index++) {
            await writeFile(join(directory, "files", crypto.randomUUID()), "abandoned");
        }
        await using bucket = await LocalBucket.open(directory);
        expect(await (await bucket.get("document"))!.text()).toBe("retained");
        expect((await readdir(join(directory, "files"))).length).toBe(2);
        await bucket.resumeMultipartUpload("multipart", uploadId).complete([selected]);
        expect(await (await bucket.get("multipart"))!.text()).toBe("retained part");
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("report reclamation failures separately from committed file replacements", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-collect-"));
    try {
        await using bucket = await LocalBucket.open(directory);
        const original = await bucket.put("document", "original");
        const path = join(directory, "files", original.version);
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
            expect(await (await bucket.get("document"))!.text()).toBe("replacement");
        } finally {
            await rm(path, { recursive: true });
            await rename(retained, path);
        }

        await bucket.collect();
        const current = await bucket.head("document");
        expect(await readdir(join(directory, "files"))).toEqual([current!.version]);
        await Promise.all([bucket[Symbol.asyncDispose](), bucket[Symbol.asyncDispose]()]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("copy files by sharing their contents until the last reference goes", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-copy-"));
    try {
        await using bucket = await LocalBucket.open(directory);
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
            etags: [copy!.etag, replaced!.etag],
            isVersionNew: copy!.version !== source.version,
            httpMetadata: [copy!.httpMetadata, replaced!.httpMetadata],
            customMetadata: [copy!.customMetadata, replaced!.customMetadata],
            storageClass: [copy!.storageClass, replaced!.storageClass],
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
        expect(await bucket.copy("source", "copy", { onlyIf: { etagMatches: "other" } })).toBe(
            null,
        );

        // keep the one content file while any copy references it
        await bucket.delete(["source", "copy"]);
        await bucket.collect();
        expect(await readdir(join(directory, "files"))).toEqual([source.version]);
        expect(await (await bucket.get("replaced"))!.text()).toBe("shared");
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
        await using bucket = await LocalBucket.open(directory);
        const cold = await bucket.put("cold", "rarely read", { storageClass: "InfrequentAccess" });
        expect({ put: cold.storageClass, head: (await bucket.head("cold"))!.storageClass }).toEqual(
            {
                put: "InfrequentAccess",
                head: "InfrequentAccess",
            },
        );
        const listed = await bucket.list();
        expect(listed.files.map((file) => file.storageClass)).toEqual(["InfrequentAccess"]);

        // refuse storage classes R2 does not offer
        await expect(
            bucket.put("glacier", "archived", { storageClass: "Glacier" as "Standard" }),
        ).rejects.toMatchObject({
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
        await using bucket = await LocalBucket.open(directory);
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
            (await bucket.head("sealed"))!.ssecKeyMd5,
            onDisk.includes(text),
        ]).toEqual([md5, md5, false]);

        // read the whole file and an unaligned range with the key
        const whole = await bucket.get("sealed", { ssecKey });
        const range = await bucket.get("sealed", { ssecKey, range: { offset: 21, length: 13 } });
        expect([await whole!.text(), await range!.text()]).toEqual([text, text.slice(21, 34)]);

        // refuse reading without the key, with another key, and a plain file with a key
        await bucket.put("plain", "open");
        const refusals = await Promise.all(
            [
                bucket.get("sealed"),
                bucket.get("sealed", { ssecKey: "b2".repeat(32) }),
                bucket.get("plain", { ssecKey }),
                bucket.put("short", "x", { ssecKey: "00" }),
            ].map((pending) =>
                pending.then(
                    () => undefined,
                    (error: { code: string; message: string }) => [error.code, error.message],
                ),
            ),
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
            await (await bucket.get("parts", { ssecKey }))!.text(),
        ]).toEqual([md5, "parted contents"]);
    } finally {
        await rm(directory, { recursive: true });
    }
});
