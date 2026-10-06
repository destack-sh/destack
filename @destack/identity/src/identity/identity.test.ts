import { expect, test } from "@destack/test";
import { IdentityError } from "../error/error.ts";
import { PublicKey } from "../key/key.ts";
import { IdentityOperation } from "./identity.ts";

test("verify an operation signed by a P-256 or an Ed25519 rotation key at its priority, and none signed by another key", async () => {
    // generate P-256, Ed25519 and unlisted rotation keys
    const [p256, ed25519, stranger] = await Promise.all([
        crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"]),
        crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign", "verify"]),
        crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"]),
    ]);
    const signingKey = await PublicKey.of(p256.publicKey);
    const rotationKeys = [signingKey, await PublicKey.of(ed25519.publicKey)];
    const operation: IdentityOperation = {
        subject: "space-1",
        previous: null,
        signingKeys: [signingKey],
        rotationKeys,
    };

    // verify the operation signed by each key at its priority
    const signed = await Promise.all(
        [p256, ed25519, stranger].map((pair) => IdentityOperation.sign(operation, pair.privateKey)),
    );
    expect([
        signed.map((each) => IdentityOperation.read(each).subject),
        await Promise.all(signed.map((each) => IdentityOperation.verify(each, rotationKeys))),
    ]).toEqual([
        ["space-1", "space-1", "space-1"],
        [0, 1, undefined],
    ]);
});

test("refuse reading a token that is no identity operation", () => {
    expect(() => IdentityOperation.read("eyJhbGciOiJub25lIn0.e30.")).toThrow(
        new IdentityError("INVALID_OPERATION", "the operation is no identity operation"),
    );
});
