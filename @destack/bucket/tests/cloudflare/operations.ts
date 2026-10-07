import assert from "node:assert/strict";
import type { R2Bucket } from "../../src/cloudflare/bucket.ts";

/** Count each R2 operation a bucket sends in the class R2 bills it in, each in exactly one take. */
export async function exerciseOperations(bucket: R2Bucket): Promise<void> {
    // write, read and list a file: two Class A and two Class B operations, the delete free
    await bucket.put("note", "first");
    await bucket.head("note");
    await (await bucket.get("note"))?.text();
    await bucket.list();
    await bucket.delete("note");
    const single = bucket.takeOperations();

    // upload a file in one part and complete it: three Class A operations
    const upload = await bucket.createMultipartUpload("document");
    const part = await upload.uploadPart(1, "only part");
    await bucket.resumeMultipartUpload(upload.key, upload.uploadId).complete([part]);
    const multipart = bucket.takeOperations();

    assert.deepStrictEqual(
        { single, multipart, after: bucket.takeOperations() },
        {
            single: { classA: 2, classB: 2 },
            multipart: { classA: 3, classB: 0 },
            after: { classA: 0, classB: 0 },
        },
    );
}
