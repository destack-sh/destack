import { expect, test } from "@destack/test";
import { ReadCache } from "./cache.ts";

test("share one read per key, store the most recent values and share nothing of a failed read", async () => {
    const cache = new ReadCache<string>(2, 1, () => 1);
    const reads: string[] = [];
    const read = (key: string) => async () => {
        reads.push(key);

        return `value of ${key}`;
    };

    // share a running read and answer stored values from memory
    const [first, shared] = await Promise.all([
        cache.get("a", [1], read("a")),
        cache.get("a", [1], read("a")),
    ]);
    expect([first, shared, await cache.get("a", [1], read("a")), reads]).toEqual([
        "value of a",
        "value of a",
        "value of a",
        ["a"],
    ]);

    // drop the least recently used value past the capacity
    await cache.get("b", [1], read("b"));
    await cache.get("a", [1], read("a"));
    await cache.get("c", [1], read("c"));
    await cache.get("a", [1], read("a"));
    await cache.get("b", [1], read("b"));
    expect(reads).toEqual(["a", "b", "c", "b"]);

    // share nothing of a failed read
    const failure = new Error("unavailable");
    await expect(cache.get("e", [1], () => Promise.reject(failure))).rejects.toBe(failure);
    expect(await cache.get("e", [1], read("e"))).toBe("value of e");
});

test("answer a value read at the same versions, reading anew once a table's version moves", async () => {
    const cache = new ReadCache<string>(2, 1, (_key, value) =>
        value.startsWith("value of big") ? 2 : 1,
    );
    const reads: string[] = [];
    const read = (key: string) => async () => {
        reads.push(key);

        return `value of ${key}`;
    };

    // read a at 1, again at 1 from memory, anew at 2, and anew once its second table moves
    await cache.get("a", [1], read("a"));
    await cache.get("a", [1], read("a"));
    await cache.get("a", [2], read("a"));
    await cache.get("a", [2], read("a"));
    await cache.get("a", [2, 1], read("a"));

    // store b at its position, and nothing the predicate refuses
    await cache.get("b", [5], read("b"));
    await cache.get("b", [5], read("b"));
    await cache.get("big", [1], read("big"));
    await cache.get("big", [1], read("big"));
    expect(reads).toEqual(["a", "a", "a", "b", "big", "big"]);
});

test("store the newer value when an older read finishes after it", async () => {
    // start a read at 1, finish a read at 2, then finish the read at 1
    const cache = new ReadCache<string>(2, 1, () => 1);
    const older = Promise.withResolvers<string>();
    const pending = cache.get("a", [1], () => older.promise);
    await cache.get("a", [2], async () => "newer");
    older.resolve("older");
    await pending;

    // answer version 2 from the newer value
    expect(await cache.get("a", [2], async () => "read again")).toBe("newer");
});

test("store values up to the cache's total weight, dropping the least recently used", async () => {
    // store rows up to three, and none heavier than two
    const cache = new ReadCache<readonly string[]>(3, 2, (_key, rows) => rows.length);
    const reads: string[] = [];
    const read = (key: string, rows: number) => async () => {
        reads.push(key);

        return Array.from({ length: rows }, () => key);
    };
    await cache.get("a", [1], read("a", 2));
    await cache.get("b", [1], read("b", 1));
    await cache.get("c", [1], read("c", 2));
    await cache.get("d", [1], read("d", 3));

    // answer b and c from memory, and read a dropped past the capacity and the heavy d again
    for (const key of ["c", "b", "a", "d"]) {
        await cache.get(key, [1], read(key, 1));
    }
    expect(reads).toEqual(["a", "b", "c", "d", "a", "d"]);
});
