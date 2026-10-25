import { createPublicKey } from "node:crypto";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { readPublicKey } from "./key.ts";

test("refuse an RSA signing key shorter than 2048 bits", async () => {
    // generate a 1024-bit RSA-PSS key and read its public PEM
    const { crypto } = testHost();
    const algorithm = {
        name: "RSA-PSS",
        hash: "SHA-256",
        modulusLength: 1024,
        publicExponent: new Uint8Array([1, 0, 1]),
    };
    const pair = await crypto.generateKey(algorithm, true, ["sign", "verify"]);
    if (!("publicKey" in pair)) {
        throw new TypeError("rsa generation returned no key pair");
    }
    const der = Buffer.from(await crypto.exportKey("spki", pair.publicKey));
    const document = createPublicKey({ key: der, format: "der", type: "spki" }).export({
        format: "pem",
        type: "spki",
    });

    expect(await refusal(() => readPublicKey(document))).toEqual([
        "BAD_REQUEST",
        "SIGNING_KEY_INVALID",
        "RSA signing keys must contain at least 2048 bits",
    ]);
});
