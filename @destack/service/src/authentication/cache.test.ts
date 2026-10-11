import { expect, test } from "@destack/test";
import { TokenCache } from "./cache.ts";
import { EXCHANGE_LIFETIME_MILLISECONDS } from "./token.ts";

test("reuse an exchanged token for its key until 80% of its life passed, and exchange again after a failure", async () => {
    // exchange under two keys, counting the exchanges
    const cache = new TokenCache();
    let issued = 0;
    const token = (key: string, now: number) =>
        cache.token(key, now, async () => `token-${++issued}`);
    const window = EXCHANGE_LIFETIME_MILLISECONDS * 0.8;

    // reuse within the window, and exchange anew past it or for another key
    const reused = [
        await token("first", 0),
        await token("first", 1000),
        await token("second", 1000),
        await token("first", window - 1),
        await token("first", window),
    ];

    // forget a failed exchange, so the next call exchanges again
    const failed = cache.token("third", 0, async () => {
        throw new Error("the exchange failed");
    });
    await expect(failed).rejects.toEqual(new Error("the exchange failed"));
    expect([...reused, await token("third", 1)]).toEqual([
        "token-1",
        "token-1",
        "token-2",
        "token-1",
        "token-3",
        "token-4",
    ]);
});
