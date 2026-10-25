import { testSimulation } from "@destack/test/world";
import { expect, refusal, test } from "@destack/test";
import { PublicKey } from "../key/key.ts";
import { Proof } from "./proof.ts";

/** The request a request proof binds. */
const REQUEST = { method: "POST", url: "https://destack.test/machines/token" };

test("verify a request proof for its subject, key, time and request, refusing every other one", async () => {
    // sign a proof of one request with a machine's key
    const world = testSimulation().environment("test");
    const pair = await world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const key = await PublicKey.of(pair.publicKey, world.crypto);
    const other = await PublicKey.of(
        (await world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign"]))
            .publicKey,
        world.crypto,
    );
    const now = world.clock.now();
    const proof = Proof.read(
        await Proof.sign(pair.privateKey, key, "machine-1", now, world, REQUEST),
    );

    // accept the request without its query and refuse every other binding
    expect([
        await refusal(
            proof.verify(key, "machine-1", now, world.crypto, {
                ...REQUEST,
                url: `${REQUEST.url}?a=1`,
            }),
        ),
        await refusal(proof.verify(other, "machine-1", now, world.crypto, REQUEST)),
        await refusal(proof.verify(key, "machine-2", now, world.crypto, REQUEST)),
        await refusal(proof.verify(key, "machine-1", now + 61_000, world.crypto, REQUEST)),
        await refusal(
            proof.verify(key, "machine-1", now, world.crypto, { ...REQUEST, method: "GET" }),
        ),
        await refusal(proof.verify(key, "machine-1", now, world.crypto)),
    ]).toEqual([
        "done",
        ["BAD_REQUEST", "PROOF_INVALID", "key proof signature is invalid"],
        ["BAD_REQUEST", "PROOF_INVALID", "key proof is for another subject or time"],
        ["BAD_REQUEST", "PROOF_INVALID", "key proof is for another subject or time"],
        ["BAD_REQUEST", "PROOF_INVALID", "key proof is for another request"],
        ["BAD_REQUEST", "PROOF_INVALID", "key proof is for another request"],
    ]);
});

test("refuse reading anything but a key proof", async () => {
    expect(await refusal(() => Proof.read("not.a.proof"))).toEqual([
        "BAD_REQUEST",
        "PROOF_INVALID",
        "key proof is not a compact JWS",
    ]);
});

test("tell a key proof from other tokens by its declared type", async () => {
    // sign a proof, and build an unsigned token of another type
    const world = testSimulation().environment("test");
    const pair = await world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const signed = await Proof.sign(
        pair.privateKey,
        await PublicKey.of(pair.publicKey, world.crypto),
        "client-1",
        world.clock.now(),
        world,
    );
    const header = btoa(JSON.stringify({ alg: "ES256", typ: "at+jwt" })).replace(/=+$/u, "");

    // take the proof alone
    expect([Proof.is(signed), Proof.is(`${header}.e30.c2ln`), Proof.is("0".repeat(64))]).toEqual([
        true,
        false,
        false,
    ]);
});
