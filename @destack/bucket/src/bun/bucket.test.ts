import { testSimulation } from "@destack/test/world";
import { present, Uuid } from "@destack/schema";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { join } from "node:path";
import { type UploadedPart, StorageClass } from "../bucket/index.ts";
import { LocalBucket } from "./index.ts";

/** The files the stress test puts, 1.6 ms each on macOS for the fsync of blob and directory. */
const STRESS_FILES = 25;

test("put, read and delete many files, keeping the blobs of the files left alone", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-stress-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);

    // put the files one at a time, read each back, then delete every other one
    const keys = Array.from(
        { length: STRESS_FILES },
        (_, index) => `file-${String(index).padStart(4, "0")}`,
    );
    for (const key of keys) {
        await bucket.put(key, `contents of ${key}`);
    }
    const read = await Promise.all(
        keys.map(async (key) => present(await bucket.get(key), key).text()),
    );
    const kept = keys.filter((_, index) => index % 2 === 0);
    await bucket.delete(keys.filter((_, index) => index % 2 === 1));
    await bucket.sweep();

    // read every file as written, and leave the kept files' blobs
    expect({
        read,
        blobs: (await world.files.readDirectory(join(directory, "files")))
            .map((entry) => entry.name)
            .toSorted(),
    }).toEqual({
        read: keys.map((key) => `contents of ${key}`),
        blobs: kept.map((key) => digestOf(`contents of ${key}`)).toSorted(),
    });
});

test("retain local files across reopen and failed streamed uploads", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-reopen-");
    // refuse creating a bucket without the scope it belongs to
    expect(await refusal(LocalBucket.open(directory, undefined, world))).toEqual([
        "NOT_FOUND",
        "NO_SUCH_BUCKET",
        `no bucket at ${directory}`,
    ]);

    {
        await using bucket = await LocalBucket.open(directory, "space-test", world);
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
        expect(
            await refusal(bucket.put("document", "changed", { sha256: "0".repeat(64) })),
        ).toEqual(["BAD_REQUEST", "BAD_DIGEST", "object checksum does not match its contents"]);
    }
    await using restored = await LocalBucket.open(directory, "space-test", world);
    const document = present(await restored.get("document"), "document");
    expect(await new Response(document.body).text()).toBe("original");
    expect(document.httpMetadata.contentType).toBe("text/plain");
    expect(
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
    ).toEqual([digestOf("original")]);
});

test("recover the catalogue after terminating its host, and sweep its interrupted upload away", async () => {
    // start a child that commits a file, then stalls inside a second upload
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-crash-");
    const module = new URL("./index.ts", import.meta.url).href;
    const bun = import.meta.resolve("@destack/environment/bun");
    const script = `
        import { LocalBucket } from ${JSON.stringify(module)};
        import { BunEnvironment } from ${JSON.stringify(bun)};
        setInterval(() => {}, 1000);
        const bucket = await LocalBucket.open(${JSON.stringify(directory)}, "space-test", BunEnvironment.system);
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
    `;
    const preload = fileURLToPath(import.meta.resolve("@destack/package/bun/preload"));
    const { executable } = world.process;
    const child = world.process.spawn(
        [executable, "--no-env-file", "--preload", preload, "--eval", script],
        { stdout: "pipe", stderr: "inherit" },
    );

    // kill the child once it stalls, reading its exit signal
    const reader = child.stdout.getReader();
    const ready = await reader.read();
    reader.releaseLock();
    child.kill("SIGKILL");
    await child.exited;
    await child.stdout.cancel();

    // reopen the bucket once the operating system releases the lock, keeping the interrupted write until a sweep
    await using restored = await LocalBucket.open(directory, "space-test", world);
    const document = present(await restored.get("document"), "document");
    const listed = async () =>
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name);
    const reopened = (await listed()).length;
    expect([
        new TextDecoder().decode(ready.value),
        child.signal,
        await new Response(document.body).text(),
        reopened,
        await restored.sweep(),
        await listed(),
    ]).toEqual(["ready\n", "SIGKILL", "committed", 2, true, [digestOf("committed")]]);
});

test("close storage immediately after consuming or cancelling a body", async () => {
    // write a two-chunk file, and read it whole in one open and cancel its body in the next
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-close-");
    const written = new Uint8Array(128 * 1024).map((_, index) => index % 251);
    const read: boolean[] = [];
    for (const operation of ["consume", "cancel"] as const) {
        await using bucket = await LocalBucket.open(directory, "space-test", world);
        await bucket.put("document", written);
        const file = present(await bucket.get("document"), "file");
        if (operation === "consume") {
            const bytes = await file.bytes();
            read.push(bytes.every((byte, index) => byte === written[index]));
        } else {
            await file.body.cancel();
        }
    }

    // read the whole file once, and close both opens without waiting on the bodies
    expect(read).toEqual([true]);
});

test("sweep multiple batches of abandoned files without deleting files or upload parts, leaving them at open", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-recovery-");
    let uploadId: string;
    let selected: UploadedPart;
    {
        await using bucket = await LocalBucket.open(directory, "space-test", world);
        await bucket.put("document", "retained");
        const upload = await bucket.createMultipartUpload("multipart");
        uploadId = upload.uploadId;
        selected = await upload.uploadPart(1, "retained part");
    }

    // leave more unreferenced blobs than one sweep query can take, and an interrupted write
    for (let index = 0; index < 501; index++) {
        await world.files.writeTextFile(
            join(directory, "files", digestOf(`abandoned ${index}`)),
            "abandoned",
        );
    }
    await world.files.writeTextFile(
        join(directory, "files", `.${Uuid.v4(world.random)}`),
        "interrupted",
    );
    await using bucket = await LocalBucket.open(directory, "space-test", world);
    const reopened = (await world.files.readDirectory(join(directory, "files"))).map(
        (entry) => entry.name,
    ).length;
    expect([
        await present(await bucket.get("document"), "document").text(),
        reopened,
        await bucket.sweep(),
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name)
            .length,
    ]).toEqual(["retained", 504, true, 2]);
    await bucket.resumeMultipartUpload("multipart", uploadId).complete([selected]);
    expect(await present(await bucket.get("multipart"), "multipart").text()).toBe("retained part");
});

test("report reclamation failures separately from committed file replacements", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-collect-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);
    await bucket.put("document", "original");
    const path = join(directory, "files", digestOf("original"));
    const retained = `${path}.retained`;

    // prevent unlinking the old version without changing the published replacement
    await world.files.rename(path, retained);
    await world.files.mkdir(path);
    await world.files.writeTextFile(join(path, "blocked"), "prevent removal");
    try {
        const replacement = await bucket.put("document", "replacement");
        expect(await bucket.head("document")).toEqual(replacement);
        expect([
            await refusal(bucket.collect()),
            await refusal(bucket.put("document", "uncommitted")),
        ]).toEqual([
            ["INTERNAL_SERVER_ERROR", "FILE_SYSTEM_FAILED", `cannot remove ${path}: ERR_FS_EISDIR`],
            ["INTERNAL_SERVER_ERROR", "FILE_SYSTEM_FAILED", `cannot remove ${path}: ERR_FS_EISDIR`],
        ]);
        expect(await present(await bucket.get("document"), "document").text()).toBe("replacement");
    } finally {
        await world.files.remove(path, { recursive: true });
        await world.files.rename(retained, path);
    }

    await bucket.collect();
    expect(
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
    ).toEqual([digestOf("replacement")]);
    await Promise.all([bucket[Symbol.asyncDispose](), bucket[Symbol.asyncDispose]()]);
});

test("copy files by sharing their contents until the last reference goes", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-copy-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);
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
    expect([copy, replaced]).toEqual([await bucket.head("copy"), await bucket.head("replaced")]);
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
    expect(await refusal(bucket.copy("missing", "copy"))).toEqual([
        "NOT_FOUND",
        "NO_SUCH_KEY",
        "the source file does not exist",
    ]);
    expect(
        await refusal(bucket.copy("source", "copy", { onlyIf: { etagMatches: "other" } })),
    ).toEqual([
        "PRECONDITION_FAILED",
        "PRECONDITION_UNMET",
        "at least one of the preconditions did not hold",
    ]);

    // keep the one blob, named by its digest, while any copy references it
    await bucket.delete(["source", "copy"]);
    await bucket.collect();
    expect(
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
    ).toEqual([digestOf("shared")]);
    expect(await present(await bucket.get("replaced"), "replaced").text()).toBe("shared");
    await bucket.delete("replaced");
    await bucket.collect();
    expect(
        (await world.files.readDirectory(join(directory, "files"))).map((entry) => entry.name),
    ).toEqual([]);
});

test("record storage classes and refuse storage classes R2 does not offer", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-class-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);
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
    expect(await refusal(Promise.try(() => StorageClass.read("Glacier")))).toEqual([
        "BAD_REQUEST",
        "INVALID_STORAGE_CLASS",
        "unknown storage class Glacier",
    ]);
});

test("encrypt files under customer keys, reading ranges with the key and refusing any other", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-customer-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);
    const ssecKey = "a1".repeat(32);
    const md5 = createHash("md5").update(Uint8Array.fromHex(ssecKey)).digest("base64");
    const text = "customer encrypted contents, longer than one AES block";

    // store the file encrypted, and keep no plaintext on disk
    const stored = await bucket.put("sealed", text, { ssecKey });
    const onDisk = await Promise.all(
        (await world.files.readDirectory(join(directory, "files")))
            .map((entry) => entry.name)
            .map((name) => world.files.readTextFile(join(directory, "files", name))),
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
        ["BAD_REQUEST", "INVALID_REQUEST", "the file is encrypted with another customer key"],
        ["BAD_REQUEST", "INVALID_REQUEST", "the file is encrypted with another customer key"],
        ["BAD_REQUEST", "INVALID_REQUEST", "the file is not encrypted with a customer key"],
        [
            "BAD_REQUEST",
            "INVALID_REQUEST",
            "a customer key must be 32 bytes or 64 hexadecimal digits",
        ],
    ]);

    // upload parts under the upload's key, refusing a part without it
    const upload = await bucket.createMultipartUpload("parts", { ssecKey });
    expect(await refusal(upload.uploadPart(1, "first"))).toEqual([
        "BAD_REQUEST",
        "INVALID_REQUEST",
        "the file is encrypted with another customer key",
    ]);
    const part = await upload.uploadPart(1, "parted contents", { ssecKey });
    const completed = await upload.complete([part]);
    expect([
        completed.ssecKeyMd5,
        await present(await bucket.get("parts", { ssecKey }), "parts").text(),
    ]).toEqual([md5, "parted contents"]);
});

/** Name a blob by the SHA-256 digest of its text. */
function digestOf(text: string): string {
    return createHash("sha256").update(text).digest("hex");
}

test("count the bytes a bucket's published files keep, a replaced file once and a deleted one not at all", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-bytes-");
    // publish two files, replace one and delete the other
    await using bucket = await LocalBucket.open(directory, "space-test", world);
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
});

test("refuse replacing or deleting a locked file until its retain-until date, then allow both", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-lock-");
    // lock a file for a moment beside an unlocked one
    await using bucket = await LocalBucket.open(directory, "space-test", world);
    const retainUntil = new Date(world.clock.now() + 200);
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
    await simulation.jump(simulation.clock.now() + 250);
    await bucket.delete("audit/segment");
    expect([replaced, deleted, held, await bucket.head("audit/segment")]).toEqual([
        ["FORBIDDEN", "ACCESS_DENIED", "file audit/segment is retained until its lock expires"],
        ["FORBIDDEN", "ACCESS_DENIED", "file audit/segment is retained until its lock expires"],
        [retainUntil.getTime(), false],
        null,
    ]);
});

test("share a bucket between two processes, deleting blobs only once the other closes", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-shared-");
    // write a file, then open the bucket in a second process writing its own and reading this one
    await using bucket = await LocalBucket.open(directory, "space-test", world);
    await bucket.put("parent", "from parent");
    const opener = fileURLToPath(new URL("./test/open.ts", import.meta.url));
    const child = world.process.spawn([world.process.executable, "run", opener, directory], {
        workingDirectory: fileURLToPath(new URL("../..", import.meta.url)),
        stdin: "pipe",
        stdout: "pipe",
        stderr: "inherit",
    });
    onTestFinished(async () => {
        child.kill("SIGKILL");
        await child.exited;
    });
    const reader = child.stdout.getReader();
    const { value } = await reader.read();
    reader.releaseLock();
    const reported = new TextDecoder().decode(value);

    // read the child's file, and keep its deleted blob while the child has the bucket open
    const read = await present(await bucket.get("child"), "the child's file").text();
    await bucket.delete(["child"]);
    const whileShared = {
        isSwept: await bucket.sweep(),
        blobs: (await world.files.readDirectory(join(directory, "files")))
            .map((entry) => entry.name)
            .toSorted(),
    };

    // delete it once the child closes the bucket
    await child.stdin.close();
    const code = await child.exited;
    const isSwept = await bucket.sweep();
    const blobs = (await world.files.readDirectory(join(directory, "files")))
        .map((entry) => entry.name)
        .toSorted();
    expect({ reported, read, whileShared, code, isSwept, blobs }).toEqual({
        reported: `${JSON.stringify({ parent: "from parent" })}\n`,
        read: "from child",
        whileShared: {
            isSwept: false,
            blobs: [digestOf("from child"), digestOf("from parent")].toSorted(),
        },
        code: 0,
        isSwept: true,
        blobs: [digestOf("from parent")],
    });
});

test("clone a plain file's bytes to a path, refusing a missing, encrypted or multipart file", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-clone-");
    await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test", world);

    // clone a plain file
    await bucket.put("plain", "plain contents");
    await bucket.clone("plain", join(directory, "cloned"));

    // refuse a missing key, an encrypted file and a multipart file
    await bucket.put("sealed", "sealed contents", { ssecKey: "a1".repeat(32) });
    const upload = await bucket.createMultipartUpload("parts");
    const first = await upload.uploadPart(1, "x".repeat(5 * 1024 * 1024));
    const second = await upload.uploadPart(2, "tail");
    await upload.complete([first, second]);
    const refusals = await Promise.all(
        [
            bucket.clone("missing", join(directory, "missing")),
            bucket.clone("sealed", join(directory, "sealed")),
            bucket.clone("parts", join(directory, "parts")),
        ].map((pending) => refusal(pending)),
    );

    expect({
        cloned: await world.files.readTextFile(join(directory, "cloned")),
        refusals,
    }).toEqual({
        cloned: "plain contents",
        refusals: [
            ["NOT_FOUND", "NO_SUCH_KEY", "no file at missing"],
            [
                "UNPROCESSABLE_CONTENT",
                "FILE_NOT_CLONABLE",
                "cloning needs a plain single-part file: sealed",
            ],
            [
                "UNPROCESSABLE_CONTENT",
                "FILE_NOT_CLONABLE",
                "cloning needs a plain single-part file: parts",
            ],
        ],
    });
});
