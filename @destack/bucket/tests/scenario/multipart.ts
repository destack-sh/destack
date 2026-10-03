import { present } from "@destack/schema";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import type { Bucket } from "../../src/index.ts";

/** Compare complete streamed contents and publication behavior across backends. */
export async function exerciseMultipart(bucket: Bucket): Promise<void> {
    await bucket.put("document", "original");
    const original = present(await bucket.get("document"), "original");
    const upload = await bucket.createMultipartUpload("document", {
        customMetadata: { format: "binary" },
    });
    await upload.uploadPart(1, "discarded");
    const first = new Uint8Array(5 * 1024 * 1024).fill(97);
    const last = new TextEncoder().encode("final bytes");
    const selected = [await upload.uploadPart(1, first), await upload.uploadPart(2, last)];
    assert.strictEqual(await present(await bucket.get("document"), "document").text(), "original");

    // complete from a fresh handle and keep the previously opened file readable
    const resumed = bucket.resumeMultipartUpload(upload.key, upload.uploadId);
    const file = await resumed.complete(selected);
    assert.strictEqual(file.size, first.length + last.length);
    assert.deepStrictEqual(file.customMetadata, { format: "binary" });
    assert.strictEqual(await original.text(), "original");
    const body = present(await bucket.get("document"), "body");
    const hash = createHash("sha256");
    for await (const bytes of body.body) {
        hash.update(bytes);
    }
    assert.strictEqual(
        hash.digest("hex"),
        createHash("sha256").update(first).update(last).digest("hex"),
    );
    assert.deepStrictEqual(await bucket.head("document"), file);

    // leave the completed file unchanged when another upload aborts
    const abandoned = await bucket.createMultipartUpload("document");
    await abandoned.uploadPart(1, "abandoned");
    await abandoned.abort();
    await abandoned.abort();
    await resumed.abort();
    assert.deepStrictEqual(await bucket.head("document"), file);
}
