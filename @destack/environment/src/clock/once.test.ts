import { expect, test } from "@destack/test";
import { Once } from "./once.ts";

test("read once, again after a failure, and again after a take", async () => {
    // count the reads, failing the first
    let reads = 0;
    const once = new Once<number>();
    const read = async () => {
        reads += 1;
        if (reads === 1) {
            throw new TypeError("the first read fails");
        }

        return reads;
    };

    // fail once, then keep the second read, then read anew after taking it
    await expect(once.get(read)).rejects.toEqual(new TypeError("the first read fails"));
    const kept = [await once.get(read), await once.get(read)];
    const current = await once.current();
    const taken = await once.take();
    const after = once.current();
    const fresh = await once.get(read);

    expect([kept, current, taken, after, fresh]).toEqual([[2, 2], 2, 2, undefined, 3]);
});
