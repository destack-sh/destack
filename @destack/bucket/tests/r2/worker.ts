import assert from "node:assert/strict";
import type * as Cloudflare from "@cloudflare/workers-types";
import { R2Bucket } from "../../src/r2/index.ts";

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

        const bucket = new R2Bucket(environment.BUCKET);
        try {
            // preserve metadata, views, checksums, and conditional writes
            const bytes = new Uint8Array([0, 65, 66, 0]);
            const original = await bucket.put("document", new DataView(bytes.buffer, 1, 2), {
                httpMetadata: { contentType: "text/plain" },
                customMetadata: { author: "alice" },
                md5: "b86fc6b051f63d73de262d4c34e3a0a9",
            });
            assert.equal(original.storageClass, "Standard");
            assert.equal(await (await bucket.get("document"))!.text(), "AB");
            assert.deepEqual((await bucket.head("document"))!.customMetadata, { author: "alice" });
            assert.equal(
                await bucket.put("document", "wrong", {
                    onlyIf: { etagDoesNotMatch: "*" },
                }),
                null,
            );
            const conditional = await bucket.get("document", {
                onlyIf: { etagMatches: "wrong" },
            });
            assert(conditional && !("body" in conditional));
            assert.equal(conditional.etag, original.etag);

            // check ranged responses and empty contents against the hosted service
            const ranged = await bucket.get("document", { range: { offset: 1, length: 1 } });
            assert.equal(await ranged!.text(), "B");
            assert.deepEqual(ranged!.range, { offset: 1, length: 1 });
            for (const [range, expected] of [
                [{ suffix: 1 }, "B"],
                [{ offset: 0, length: 100 }, "AB"],
                [{ length: 1 }, "A"],
            ] as const) {
                try {
                    const file = await bucket.get("document", { range });
                    assert.equal(await file!.text(), expected);
                } catch (error) {
                    throw new Error(`range ${JSON.stringify(range)}: ${String(error)}`);
                }
            }
            await assert.rejects(async () => bucket.get("document", { range: { offset: 2 } }), {
                code: "INVALID_RANGE",
                message: "the requested file range is not satisfiable",
            });
            await bucket.put("empty", null);
            assert.equal(await (await bucket.get("empty"))!.text(), "");

            // follow opaque cursors across grouped listings
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

            // resume, replace, and complete a multipart upload with each part stored in R2
            const upload = await bucket.createMultipartUpload("multipart");
            try {
                const contents = new Uint8Array(5 * 1024 * 1024).fill(65);
                await upload.uploadPart(1, "replaced");
                const first = await upload.uploadPart(1, contents);
                const last = await upload.uploadPart(2, "tail");
                const resumed = bucket.resumeMultipartUpload(upload.key, upload.uploadId);
                const completed = await resumed.complete([first, last]);
                assert.equal(completed.size, contents.length + 4);
                const restored = await (await bucket.get("multipart"))!.bytes();
                const [expected, actual] = await Promise.all([
                    crypto.subtle.digest("SHA-256", contents),
                    crypto.subtle.digest("SHA-256", restored.subarray(0, contents.length)),
                ]);
                assert.deepEqual(new Uint8Array(actual), new Uint8Array(expected));
                assert.equal(new TextDecoder().decode(restored.subarray(contents.length)), "tail");
            } finally {
                await upload.abort();
            }

            return Response.json({ status: "passed" });
        } catch (error) {
            return Response.json({ error: String(error) }, { status: 500 });
        } finally {
            // remove every test file so the disposable bucket can be deleted
            let page;
            do {
                page = await bucket.list();
                if (page.files.length) {
                    await bucket.delete(page.files.map((file) => file.key));
                }
            } while (page.truncated);
        }
    },
};
