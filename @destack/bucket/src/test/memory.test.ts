import { expect, refusal, test } from "@destack/test";
import { MemoryBucket } from "./memory.ts";
import { VirtualClock } from "@destack/environment/simulation";

test("keep files in memory as a host does: conditional reads and writes, ranges, locks until they lapse, and pages that list each delimited prefix once", async () => {
    // write three files, one locked for a minute
    const clock = new VirtualClock(Date.UTC(2026, 9, 1));
    const bucket = new MemoryBucket(clock);
    const first = await bucket.put("a/1", "hello");
    await bucket.put("a/2", "world", { retainUntil: new Date(clock.now() + 60_000) });
    await bucket.put("b", "!");

    // read a range, a failed precondition and a refused write
    const range = await (await bucket.get("a/1", { range: { offset: 1, length: 3 } }))?.text();
    const unchanged = await refusal(
        bucket.get("a/1", { onlyIf: { etagDoesNotMatch: first.etag } }),
    );
    const refusedWrite = await refusal(
        bucket.put("a/1", "again", { onlyIf: { etagMatches: "other" } }),
    );

    // list one entry a page, the delimited prefix once
    const pages: string[][] = [];
    let cursor: string | undefined;
    do {
        const page = await bucket.list({
            delimiter: "/",
            limit: 1,
            ...(cursor === undefined ? {} : { cursor }),
        });
        pages.push([...page.delimitedPrefixes, ...page.files.map((file) => file.key)]);
        cursor = page.truncated ? page.cursor : undefined;
    } while (cursor !== undefined);

    // refuse deleting the locked file until its lock lapses
    const locked = await refusal(bucket.delete("a/2"));
    clock.advance(60_000);
    await bucket.delete("a/2");

    expect({
        range,
        unchanged: unchanged[0],
        refusedWrite: refusedWrite[0],
        pages,
        locked,
        deleted: await bucket.head("a/2"),
    }).toEqual({
        range: "ell",
        unchanged: "PRECONDITION_FAILED",
        refusedWrite: "PRECONDITION_FAILED",
        pages: [["a/"], ["b"]],
        locked: ["FORBIDDEN", "ACCESS_DENIED", "file a/2 is locked"],
        deleted: null,
    });
});

test("record the checksum a write supplies, and refuse one its bytes do not match", async () => {
    const bucket = new MemoryBucket(new VirtualClock(Date.UTC(2026, 9, 1)));
    const sha256 = new Bun.CryptoHasher("sha256").update("hello").digest("hex");

    const stored = await bucket.put("a", "hello", { sha256 });
    expect([
        stored.digest.toJSON().sha256,
        await refusal(bucket.put("b", "hello", { sha256: "0".repeat(64) })),
    ]).toEqual([
        sha256,
        ["BAD_REQUEST", "BAD_DIGEST", "object checksum does not match its contents"],
    ]);
});
