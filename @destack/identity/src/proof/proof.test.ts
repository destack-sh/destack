import { expect, refusal, test } from "@destack/test";
import { PublicKey } from "../key/key.ts";
import { Proof } from "./proof.ts";

/** The request a request proof binds. */
const REQUEST = { method: "POST", url: "https://destack.test/machines/token" };

test("verify a request proof for its subject, key, time and request, refusing every other one", async () => {
    // sign a proof of one request with a machine's key
    const pair = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const key = await PublicKey.of(pair.publicKey);
    const other = await PublicKey.of(
        (await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign"]))
            .publicKey,
    );
    const now = Date.now();
    const proof = Proof.read(await Proof.sign(pair.privateKey, key, "machine-1", now, REQUEST));

    // accept the request without its query and refuse every other binding
    expect([
        await refusal(
            proof.verify(key, "machine-1", now, { ...REQUEST, url: `${REQUEST.url}?a=1` }),
        ),
        await refusal(proof.verify(other, "machine-1", now, REQUEST)),
        await refusal(proof.verify(key, "machine-2", now, REQUEST)),
        await refusal(proof.verify(key, "machine-1", now + 61_000, REQUEST)),
        await refusal(proof.verify(key, "machine-1", now, { ...REQUEST, method: "GET" })),
        await refusal(proof.verify(key, "machine-1", now)),
    ]).toEqual([
        "done",
        ["BAD_REQUEST", "INVALID_PROOF", "key proof signature is invalid"],
        ["BAD_REQUEST", "INVALID_PROOF", "key proof is for another subject or time"],
        ["BAD_REQUEST", "INVALID_PROOF", "key proof is for another subject or time"],
        ["BAD_REQUEST", "INVALID_PROOF", "key proof is for another request"],
        ["BAD_REQUEST", "INVALID_PROOF", "key proof is for another request"],
    ]);
});

test("refuse reading anything but a key proof", async () => {
    expect(await refusal(() => Proof.read("not.a.proof"))).toEqual([
        "BAD_REQUEST",
        "INVALID_PROOF",
        "key proof is not a compact JWS",
    ]);
});

test("tell a key proof from other tokens by its declared type", async () => {
    // sign a proof, and build an unsigned token of another type
    const pair = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const signed = await Proof.sign(
        pair.privateKey,
        await PublicKey.of(pair.publicKey),
        "client-1",
        Date.now(),
    );
    const header = btoa(JSON.stringify({ alg: "ES256", typ: "at+jwt" })).replace(/=+$/u, "");

    // take the proof alone
    expect([Proof.is(signed), Proof.is(`${header}.e30.c2ln`), Proof.is("0".repeat(64))]).toEqual([
        true,
        false,
        false,
    ]);
});
