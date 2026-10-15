import { MemoryKeychain } from "@destack/identity";
import { Identifier } from "@destack/schema";
import { expect, refusal, test, TestClock } from "@destack/test";
import { ClientKey } from "./client-key.ts";

test("prove a native client to its computer's machine by its kept key, refusing a forgotten or stale one", async () => {
    // keep a key for a registered client and prove it now
    const clock = new TestClock(Date.UTC(2026, 9, 14));
    const keys = new ClientKey(new MemoryKeychain(), clock);
    const client = Identifier.create("client");
    await keys.generate(client);
    const proof = await keys.prove(client);
    const verified = await keys.verify(proof);

    // refuse the proof once stale, and once the key is forgotten
    clock.advance(10 * 60 * 1000);
    const stale = await refusal(keys.verify(proof));
    await keys.forget(client);
    const forgotten = await refusal(keys.verify(proof));

    expect([verified, stale, forgotten]).toEqual([
        client,
        ["INVALID_PROOF", "key proof is for another subject or time"],
        ["UNAUTHORIZED", "no client of this computer signed it"],
    ]);
});
