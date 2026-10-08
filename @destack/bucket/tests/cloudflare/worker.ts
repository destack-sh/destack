import { present } from "@destack/schema";
import assert from "node:assert/strict";
import type * as Cloudflare from "@cloudflare/workers-types";
import { BucketPreconditionError } from "../../src/error/index.ts";
import { R2Bucket } from "../../src/cloudflare/bucket.ts";

/** The disposable bucket and test authorization supplied at deployment. */
interface Environment {
    /** The disposable R2 bucket. */
    BUCKET: Cloudflare.R2Bucket;
    /** The bearer token authorizing test requests. */
    TOKEN: string;
}

/** The Worker running the hosted adapter checks. */
export default {
    /** Exercise the adapter inside a hosted Worker. */
    async fetch(request: Request, environment: Environment): Promise<Response> {
        if (request.headers.get("authorization") !== `Bearer ${environment.TOKEN}`) {
            return new Response(null, { status: 401 });
        }
        if (request.method !== "POST") {
            return new Response(null, { status: 405 });
        }

        // exercise the adapter, removing every test file after
        const bucket = new R2Bucket(environment.BUCKET);
        try {
            await exerciseMetadata(bucket);
            await exerciseRanges(bucket);
            await exerciseListing(bucket);
            await exerciseMultipart(bucket);

            return Response.json({ status: "passed" });
        } catch (error) {
            return Response.json({ error: String(error) }, { status: 500 });
        } finally {
            await clear(bucket);
        }
    },
};

/** Preserve metadata, views, content digests and conditional writes. */
async function exerciseMetadata(bucket: R2Bucket): Promise<void> {
    const bytes = new Uint8Array([0, 65, 66, 0]);
    const original = await bucket.put("document", new DataView(bytes.buffer, 1, 2), {
        httpMetadata: { contentType: "text/plain" },
        customMetadata: { author: "alice" },
        md5: "b86fc6b051f63d73de262d4c34e3a0a9",
    });
    assert.equal(original.storageClass, "Standard");
    assert.equal(await present(await bucket.get("document"), "document").text(), "AB");
    assert.deepEqual(present(await bucket.head("document"), "document").customMetadata, {
        author: "alice",
    });
    await assert.rejects(
        bucket.put("document", "wrong", { onlyIf: { etagDoesNotMatch: "*" } }),
        (error: unknown) =>
            error instanceof BucketPreconditionError && error.current?.etag === original.etag,
    );
    await assert.rejects(
        bucket.get("document", { onlyIf: { etagMatches: "wrong" } }),
        (error: unknown) =>
            error instanceof BucketPreconditionError && error.current?.etag === original.etag,
    );
}

/** Check ranged responses and empty contents against the hosted service. */
async function exerciseRanges(bucket: R2Bucket): Promise<void> {
    const ranged = present(
        await bucket.get("document", { range: { offset: 1, length: 1 } }),
        "ranged",
    );
    assert.equal(await ranged.text(), "B");
    assert.deepEqual(ranged.range, { offset: 1, length: 1 });
    for (const [range, expected] of [
        [{ suffix: 1 }, "B"],
        [{ offset: 0, length: 100 }, "AB"],
        [{ length: 1 }, "A"],
    ] as const) {
        try {
            const file = present(await bucket.get("document", { range }), "file");
            assert.equal(await file.text(), expected);
        } catch (error) {
            throw new Error(`range ${JSON.stringify(range)}: ${String(error)}`, {
                cause: error,
            });
        }
    }
    await assert.rejects(async () => bucket.get("document", { range: { offset: 2 } }), {
        code: "INVALID_RANGE",
        message: "the requested file range is not satisfiable",
    });
    await bucket.put("empty", null);
    assert.equal(await present(await bucket.get("empty"), "empty").text(), "");
}

/** Follow opaque cursors across grouped listings. */
async function exerciseListing(bucket: R2Bucket): Promise<void> {
    await bucket.put("files/a/one", "one");
    await bucket.put("files/b/two", "two");
    const page = await bucket.list({ prefix: "files/", delimiter: "/", limit: 1 });
    assert.deepEqual(page.delimitedPrefixes, ["files/a/"]);
    assert.equal(page.truncated, true);
    const next = await bucket.list({
        prefix: "files/",
        delimiter: "/",
        cursor: page.cursor,
    });
    assert.deepEqual(next.delimitedPrefixes, ["files/b/"]);
    assert.equal(next.truncated, false);
}

/** Resume, replace and complete a multipart upload with each part stored in R2. */
async function exerciseMultipart(bucket: R2Bucket): Promise<void> {
    const upload = await bucket.createMultipartUpload("multipart");
    try {
        const contents = new Uint8Array(5 * 1024 * 1024).fill(65);
        await upload.uploadPart(1, "replaced");
        const first = await upload.uploadPart(1, contents);
        const last = await upload.uploadPart(2, "tail");
        const resumed = bucket.resumeMultipartUpload(upload.key, upload.uploadId);
        const completed = await resumed.complete([first, last]);
        assert.equal(completed.size, contents.length + 4);
        const restored = await present(await bucket.get("multipart"), "multipart").bytes();
        const [expected, actual] = await Promise.all([
            crypto.subtle.digest("SHA-256", contents),
            crypto.subtle.digest("SHA-256", restored.subarray(0, contents.length)),
        ]);
        assert.deepEqual(new Uint8Array(actual), new Uint8Array(expected));
        assert.equal(new TextDecoder().decode(restored.subarray(contents.length)), "tail");
    } finally {
        await upload.abort();
    }
}

/** Remove every test file, so the disposable bucket can be deleted. */
async function clear(bucket: R2Bucket): Promise<void> {
    let page;
    do {
        page = await bucket.list();
        if (page.files.length) {
            await bucket.delete(page.files.map((file) => file.key));
        }
    } while (page.truncated);
}
