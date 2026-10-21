import { expect, test } from "@destack/test";
import { TokenCache } from "./cache.ts";

test("reuse an exchanged token for its key until 80% of its life passed, and exchange again after a failure", async () => {
    // exchange minute-long tokens under two keys, counting the exchanges
    const cache = new TokenCache();
    let issued = 0;
    const token = (key: string, now: number) =>
        cache.token(key, now, async () => ({
            accessToken: `token-${++issued}`,
            expiresAt: now + 60_000,
        }));

    // reuse within the window, and exchange anew past it or for another key
    const reused = [
        await token("first", 0),
        await token("first", 1000),
        await token("second", 1000),
        await token("first", 47_999),
        await token("first", 48_000),
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

test("exchange anew after a step-up instead of reusing the lower-assurance token", async () => {
    // exchange a token living for hours for a plain call, counting the exchanges
    const cache = new TokenCache();
    let issued = 0;
    const exchange = async () => ({ accessToken: `token-${++issued}`, expiresAt: 10_000_000 });
    const plain = await cache.token("space", 0, exchange);

    // exchange again for a call asking a second factor, then reuse that token for it and for a plain call
    const stepUp = { assurance: 2, maxAge: 900_000 };
    const elevated = await cache.token("space", 1000, exchange, stepUp);
    const reused = [
        await cache.token("space", 2000, exchange, stepUp),
        await cache.token("space", 3000, exchange),
    ];

    // exchange anew once the elevation is older than the step-up allows
    const aged = await cache.token("space", 1000 + 900_001, exchange, stepUp);
    expect([plain, elevated, ...reused, aged]).toEqual([
        "token-1",
        "token-2",
        "token-2",
        "token-2",
        "token-3",
    ]);
});
