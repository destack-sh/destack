import { expect, refusal, test } from "@destack/test";
import { MemoryBucket } from "./memory.ts";

test("keep files in memory as a host does: conditional reads and writes, ranges, locks until they lapse, and pages that list each delimited prefix once", async () => {
    // write three files, one locked for a minute
    let now = Date.UTC(2026, 9, 1);
    const bucket = new MemoryBucket(() => now);
    const first = await bucket.put("a/1", "hello");
    await bucket.put("a/2", "world", { retainUntil: new Date(now + 60_000) });
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
    now += 60_000;
    await bucket.delete("a/2");

    expect({
        range,
        unchanged: unchanged[0],
        refusedWrite: refusedWrite[0],
        pages,
        locked: locked[0],
        deleted: await bucket.head("a/2"),
    }).toEqual({
        range: "ell",
        unchanged: "PRECONDITION_FAILED",
        refusedWrite: "PRECONDITION_FAILED",
        pages: [["a/"], ["b"]],
        locked: "LOCKED",
        deleted: null,
    });
});
