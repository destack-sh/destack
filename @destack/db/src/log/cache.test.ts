import { expect, test } from "@destack/test";
import { ReadCache } from "./cache.ts";

test("share one read per key, keep the most recent values and share nothing of a failed read", async () => {
    const cache = new ReadCache<string>(2, 1, () => 1);
    const reads: string[] = [];
    const read = (key: string) => async () => {
        reads.push(key);

        return `value of ${key}`;
    };

    // share a running read and answer kept values from memory
    const [first, shared] = await Promise.all([
        cache.get("a", read("a"), at(1)),
        cache.get("a", read("a"), at(1)),
    ]);
    expect([first, shared, await cache.get("a", read("a"), at(1)), reads]).toEqual([
        "value of a",
        "value of a",
        "value of a",
        ["a"],
    ]);

    // drop the least recently used value past the capacity
    await cache.get("b", read("b"), at(1));
    await cache.get("a", read("a"), at(1));
    await cache.get("c", read("c"), at(1));
    await cache.get("a", read("a"), at(1));
    await cache.get("b", read("b"), at(1));
    expect(reads).toEqual(["a", "b", "c", "b"]);

    // share nothing of a failed read
    const failure = new Error("unavailable");
    await expect(cache.get("e", () => Promise.reject(failure), at(1))).rejects.toBe(failure);
    expect(await cache.get("e", read("e"), at(1))).toBe("value of e");
});

test("answer a position from a value read at it or later, reading a later position or epoch anew", async () => {
    const cache = new ReadCache<string>(2, 1, (value) =>
        value.startsWith("value of big") ? 2 : 1,
    );
    const reads: string[] = [];
    const read = (key: string) => async () => {
        reads.push(key);

        return `value of ${key}`;
    };

    // read a at 1, then at 2, answer 1 and 2 from the later value, and read a later epoch anew
    await cache.get("a", read("a"), at(1));
    await cache.get("a", read("a"), at(1));
    await cache.get("a", read("a"), at(2));
    await cache.get("a", read("a"), at(1));
    await cache.get("a", read("a"), at(2));
    await cache.get("a", read("a"), { log: "l1", epoch: "e2", sequence: 1 });

    // keep b at its position, and nothing the predicate refuses
    await cache.get("b", read("b"), at(5));
    await cache.get("b", read("b"), at(5));
    await cache.get("big", read("big"), at(1));
    await cache.get("big", read("big"), at(1));
    expect(reads).toEqual(["a", "a", "a", "b", "big", "big"]);
});

test("keep the newer value when an older read finishes after it", async () => {
    // start a read at 1, finish a read at 2, then finish the read at 1
    const cache = new ReadCache<string>(2, 1, () => 1);
    const older = Promise.withResolvers<string>();
    const pending = cache.get("a", () => older.promise, at(1));
    await cache.get("a", async () => "newer", at(2));
    older.resolve("older");
    await pending;

    // answer position 2 from the newer value
    expect(await cache.get("a", async () => "read again", at(2))).toBe("newer");
});

test("keep values up to the cache's total weight, dropping the least recently used", async () => {
    // keep rows up to three, and none heavier than two
    const cache = new ReadCache<readonly string[]>(3, 2, (rows) => rows.length);
    const reads: string[] = [];
    const read = (key: string, rows: number) => async () => {
        reads.push(key);

        return Array.from({ length: rows }, () => key);
    };
    await cache.get("a", read("a", 2), at(1));
    await cache.get("b", read("b", 1), at(1));
    await cache.get("c", read("c", 2), at(1));
    await cache.get("d", read("d", 3), at(1));

    // answer b and c from memory, and read a dropped past the capacity and the heavy d again
    for (const key of ["c", "b", "a", "d"]) {
        await cache.get(key, read(key, 1), at(1));
    }
    expect(reads).toEqual(["a", "b", "c", "d", "a", "d"]);
});

/** Build a position in the first epoch. */
function at(sequence: number) {
    return { log: "l1", epoch: "e1", sequence };
}
