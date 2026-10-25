import { testSimulation } from "@destack/object/test";
import { MemoryKeychain } from "@destack/identity";
import { Identifier } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { ClientKey } from "./client-key.ts";

test("prove a client to its computer's machine by its stored key, refusing a forgotten or stale one", async () => {
    // store a key for a registered client and prove it now
    const simulation = testSimulation(Date.UTC(2026, 9, 14));
    const { clock } = simulation;
    const world = simulation.environment("test");
    const keys = new ClientKey(new MemoryKeychain(), world);
    const client = Identifier.create("client", world);
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
        ["BAD_REQUEST", "PROOF_INVALID", "key proof is for another subject or time"],
        ["UNAUTHORIZED", "CLIENT_KEY_UNKNOWN", "no client of this computer signed it"],
    ]);
});
