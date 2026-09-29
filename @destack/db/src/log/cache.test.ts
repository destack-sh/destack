import { expect, test } from "@destack/test";
import { ReadCache } from "./cache.ts";

test("share one read per key, keep the most recent values and drop forgotten reads", async () => {
    const cache = new ReadCache<string>(2);
    const reads: string[] = [];
    const read = (key: string) => async () => {
        reads.push(key);

        return `value of ${key}`;
    };

    // share a running read and answer kept values from memory
    const [first, shared] = await Promise.all([
        cache.get("a", read("a")),
        cache.get("a", read("a")),
    ]);
    expect([first, shared, await cache.get("a", read("a")), reads]).toEqual([
        "value of a",
        "value of a",
        "value of a",
        ["a"],
    ]);

    // drop the least recently used value past the capacity
    await cache.get("b", read("b"));
    await cache.get("a", read("a"));
    await cache.get("c", read("c"));
    await cache.get("a", read("a"));
    await cache.get("b", read("b"));
    expect(reads).toEqual(["a", "b", "c", "b"]);

    // leave a read forgotten while running out of the cache
    const pending = Promise.withResolvers<string>();
    const running = cache.get("d", () => pending.promise);
    cache.forget("d");
    pending.resolve("stale");
    await running;
    expect(await cache.get("d", read("d"))).toBe("value of d");

    // share nothing of a failed read
    const failure = new Error("unavailable");
    await expect(cache.get("e", () => Promise.reject(failure))).rejects.toBe(failure);
    expect(await cache.get("e", read("e"))).toBe("value of e");
});
