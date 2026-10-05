import { present } from "@destack/schema";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import type { Bucket } from "../../src/index.ts";
import { CHECKSUM_ALGORITHMS } from "../../src/index.ts";

/** Retain exact body bytes, HTTP metadata, and content checksums. */
export async function exerciseContents(bucket: Bucket): Promise<void> {
    assert.strictEqual(await bucket.head("missing"), null);
    assert.strictEqual(await bucket.get("missing"), null);
    const empty = await bucket.put("empty", null);
    assert.strictEqual(empty.size, 0);
    const emptyBody = present(await bucket.get("empty"), "emptyBody");
    assert.strictEqual(emptyBody.bodyUsed, false);
    assert.strictEqual(await emptyBody.text(), "");
    assert.strictEqual(emptyBody.bodyUsed, true);
    await bucket.delete("empty");

    // preserve byte-view boundaries and stored HTTP metadata
    const bytes = new Uint8Array([0, 65, 66, 0]);
    await bucket.put("view", new DataView(bytes.buffer, 1, 2), {
        httpMetadata: { contentType: "text/plain", cacheControl: "no-cache" },
    });
    const view = present(await bucket.get("view"), "view");
    assert.strictEqual(await view.text(), "AB");
    assert.deepStrictEqual(
        Object.fromEntries(
            Object.entries(view.httpMetadata).filter(([, value]) => value !== undefined),
        ),
        {
            contentType: "text/plain",
            cacheControl: "no-cache",
        },
    );
    await bucket.delete("view");

    // keep supplied checksums that describe exactly the accepted contents
    const value = new TextEncoder().encode("checksummed contents");
    const md5 = createHash("md5").update(value).digest("hex");
    for (const algorithm of CHECKSUM_ALGORITHMS) {
        const digest = createHash(algorithm).update(value).digest("hex");
        const file = await bucket.put("checksum", value, { [algorithm]: digest });
        assert.deepStrictEqual(JSON.parse(JSON.stringify(file.checksums)), {
            md5,
            [algorithm]: digest,
        });
        const restored = present(await bucket.get("checksum"), "restored");
        assert.deepStrictEqual(restored.checksums.toJSON(), file.checksums.toJSON());
        assert.deepStrictEqual(new Uint8Array(await restored.arrayBuffer()), value);
    }
    await bucket.delete("checksum");
}

/**
 * Apply conditional updates and ranges while keeping opened content readable.
 *
 * The local R2 simulator reports unsatisfiable ranges without R2's error code 10039, so the hosted test:cloudflare run checks that failure there.
 */
export async function exerciseConditions(bucket: Bucket, isSimulatedR2 = false): Promise<void> {
    // publish immutable content and retain metadata through listing and reads
    const first = present(
        await bucket.put("📁/one", "abcdef", {
            onlyIf: { etagDoesNotMatch: "*" },
            httpMetadata: {
                contentType: "text/plain",
                contentLanguage: "en",
                contentDisposition: "attachment; filename=one.txt",
                contentEncoding: "identity",
                cacheControl: "max-age=60",
                cacheExpiry: new Date("2030-01-01T00:00:00.000Z"),
            },
            customMetadata: { owner: "alice" },
        }),
        "first",
    );
    assert.deepStrictEqual(await bucket.head("📁/one"), first);
    const headers = new Headers();
    first.writeHttpMetadata(headers);
    assert.deepStrictEqual(Object.fromEntries(headers), {
        "cache-control": "max-age=60",
        "content-disposition": "attachment; filename=one.txt",
        "content-encoding": "identity",
        "content-language": "en",
        "content-type": "text/plain",
        expires: "Tue, 01 Jan 2030 00:00:00 GMT",
    });
    assert.strictEqual(first.httpEtag, `"${first.etag}"`);
    assert.strictEqual(first.uploaded instanceof Date, true);

    // apply the same date and entity tag precedence in both implementations
    const earlier = new Date(first.uploaded.getTime() - 1000);
    const later = new Date(first.uploaded.getTime() + 1000);
    assert.deepStrictEqual(await bucket.get("📁/one", { onlyIf: { uploadedAfter: later } }), first);
    assert.deepStrictEqual(
        await bucket.get("📁/one", { onlyIf: { uploadedBefore: earlier } }),
        first,
    );
    const matching = await bucket.get("📁/one", {
        onlyIf: { etagMatches: first.etag, uploadedBefore: earlier },
    });
    assert.strictEqual(matching && "body" in matching ? await matching.text() : matching, "abcdef");
    assert.strictEqual(
        await bucket.put("📁/one", "wrong", { onlyIf: { uploadedAfter: later } }),
        null,
    );
    const retained = present(await bucket.get("📁/one"), "retained");
    const sliced = present(
        await bucket.get("📁/one", { range: { offset: 1, length: 3 } }),
        "sliced",
    );
    assert.strictEqual(await new Response(sliced.body).text(), "bcd");
    assert.deepStrictEqual(sliced.range, { offset: 1, length: 3 });
    const suffix = present(await bucket.get("📁/one", { range: { suffix: 2 } }), "suffix");
    assert.strictEqual(await new Response(suffix.body).text(), "ef");
    const prefix = present(await bucket.get("📁/one", { range: { length: 2 } }), "prefix");
    assert.strictEqual(await prefix.text(), "ab");

    // clamp a range reaching past the end
    const clamped = present(
        await bucket.get("📁/one", { range: { offset: 4, length: 100 } }),
        "clamped",
    );
    assert.strictEqual(await clamped.text(), "ef");

    // refuse a range starting at the end, which the simulator reports without R2's code
    if (!isSimulatedR2) {
        await assert.rejects(async () => bucket.get("📁/one", { range: { offset: 6 } }), {
            code: "INVALID_RANGE",
            message: "the requested file range is not satisfiable",
        });
    }
    await assert.rejects(async () => bucket.get("📁/one", { range: { offset: -1 } }), {
        code: "INVALID_RANGE",
        message: "the requested file range is not satisfiable",
    });

    // preserve the complete previous file when a precondition fails
    assert.strictEqual(
        await bucket.put("📁/one", "wrong", { onlyIf: { etagDoesNotMatch: "*" } }),
        null,
    );
    assert.deepStrictEqual(await bucket.get("📁/one", { onlyIf: { etagMatches: "wrong" } }), first);
    assert.deepStrictEqual(await bucket.head("📁/one"), first);

    // preserve absent keys under conditional operations
    assert.strictEqual(await bucket.get("missing", { onlyIf: { etagMatches: "*" } }), null);
    assert.strictEqual(
        await bucket.put("missing", "wrong", { onlyIf: { etagMatches: "*" } }),
        null,
    );
    assert.strictEqual(await bucket.head("missing"), null);

    // accept one of two competing replacements of the previous entity tag
    const replacements = await Promise.all([
        bucket.put("📁/one", "second", { onlyIf: { etagMatches: first.etag } }),
        bucket.put("📁/one", "third", { onlyIf: { etagMatches: first.etag } }),
    ]);
    assert.deepStrictEqual(
        replacements.filter((entry) => entry === null),
        [null],
    );
    assert.deepStrictEqual(
        replacements.filter((entry) => entry !== null),
        [await bucket.head("📁/one")],
    );
    assert.strictEqual(await new Response(retained.body).text(), "abcdef");
}

/** Page literal file keys and delete exact selections. */
export async function exerciseListing(bucket: Bucket): Promise<void> {
    await bucket.put("📁/one", "one");
    // match key prefixes as literal UTF-8 strings, apart from filesystem paths and SQL patterns
    await bucket.put("📁/two", "two");
    await bucket.put("../outside", "safe file key");
    const page = await bucket.list({
        prefix: "📁/",
        limit: 1,
        include: ["httpMetadata", "customMetadata"],
    });
    assert.deepStrictEqual(page.files, [await bucket.head("📁/one")]);
    assert.strictEqual(page.truncated, true);
    await assert.rejects(async () => bucket.list({ prefix: "other/", cursor: page.cursor }), {
        code: "INVALID_CURSOR",
    });
    const last = await bucket.list({ prefix: "📁/", cursor: page.cursor, limit: 1 });
    assert.deepStrictEqual(last, {
        files: [await bucket.head("📁/two")],
        delimitedPrefixes: [],
        truncated: false,
    });
    const after = await bucket.list({ prefix: "📁/", startAfter: "📁/one" });
    assert.deepStrictEqual(
        after.files.map((file) => file.key),
        ["📁/two"],
    );
    await bucket.delete("📁/one");
    await bucket.delete("📁/one");
    assert.strictEqual(await bucket.get("📁/one"), null);
    await bucket.delete(["📁/two", "../outside", "missing", "📁/two"]);
    assert.deepStrictEqual(await bucket.list(), {
        files: [],
        delimitedPrefixes: [],
        truncated: false,
    });
}

/** Resume grouped listings after new keys enter an earlier group. */
export async function exerciseGroups(bucket: Bucket): Promise<void> {
    // skip every key in an emitted prefix, including new members
    for (const key of ["files/a/one", "files/a/two", "files/b/one", "files/root"]) {
        await bucket.put(key, key);
    }
    const group = await bucket.list({ prefix: "files/", delimiter: "/", limit: 1 });
    assert.strictEqual(group.truncated, true);
    assert.deepStrictEqual(
        { ...group, cursor: undefined },
        {
            files: [],
            delimitedPrefixes: ["files/a/"],
            truncated: true,
            cursor: undefined,
        },
    );
    await bucket.put("files/a/three", "added between pages");
    const nextGroup = await bucket.list({
        prefix: "files/",
        delimiter: "/",
        limit: 1,
        cursor: group.cursor,
    });
    assert.strictEqual(nextGroup.truncated, true);
    assert.deepStrictEqual(
        { ...nextGroup, cursor: undefined },
        {
            files: [],
            delimitedPrefixes: ["files/b/"],
            truncated: true,
            cursor: undefined,
        },
    );
    const leaf = await bucket.list({
        prefix: "files/",
        delimiter: "/",
        limit: 1,
        cursor: nextGroup.cursor,
    });
    assert.deepStrictEqual(leaf, {
        files: [await bucket.head("files/root")],
        delimitedPrefixes: [],
        truncated: false,
    });
    await assert.rejects(
        async () => bucket.list({ prefix: "files/", delimiter: "-", cursor: group.cursor }),
        {
            code: "INVALID_CURSOR",
            message: "invalid file listing cursor",
        },
    );
}

/** Page past a folder marker key equal to the prefix, then through the files and groups after it. */
export async function exerciseMarkers(bucket: Bucket): Promise<void> {
    // list the marker, the file beside it and the group below it one page at a time
    for (const key of ["marks/", "marks/b", "marks/c/d"]) {
        await bucket.put(key, key);
    }
    const pages = [];
    let cursor: string | undefined;
    do {
        const page = await bucket.list({
            prefix: "marks/",
            delimiter: "/",
            limit: 1,
            ...(cursor === undefined ? {} : { cursor }),
        });
        pages.push([page.files.map((entry) => entry.key), page.delimitedPrefixes, page.truncated]);
        cursor = page.cursor;
    } while (cursor !== undefined);
    assert.deepStrictEqual(pages, [
        [["marks/"], [], true],
        [["marks/b"], [], true],
        [[], ["marks/c/"], false],
    ]);
}
