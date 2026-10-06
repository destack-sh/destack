import { expect, test } from "@destack/test";
import { IdentityError } from "../error/error.ts";
import { PublicKey } from "../key/key.ts";
import { Proof } from "./proof.ts";

/** The request a request proof binds. */
const REQUEST = { method: "POST", url: "https://destack.test/machines/token" };

/** Report a verification's outcome: accepted, or the refusal's message. */
function refusal(verifying: Promise<void>): Promise<string> {
    return verifying.then(
        () => "accepted",
        (error: IdentityError) => error.message,
    );
}

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
        "accepted",
        "key proof signature is invalid",
        "key proof is for another subject or time",
        "key proof is for another subject or time",
        "key proof is for another request",
        "key proof is for another request",
    ]);
});

test("refuse reading anything but a key proof", () => {
    expect(() => Proof.read("not.a.proof")).toThrow(
        new IdentityError("INVALID_PROOF", "key proof is no compact JWS"),
    );
});
