import { Simulation } from "@destack/environment/simulation";
import { expect, test, refusal } from "@destack/test";
import { PublicKey } from "../key/key.ts";
import { IdentityOperation } from "./identity.ts";

test("verify an operation signed by a P-256 or an Ed25519 rotation key at its priority, and none signed by another key", async () => {
    // generate P-256, Ed25519 and unlisted rotation keys
    const world = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 24) }).environment("test");
    const [p256, ed25519, stranger] = await Promise.all([
        world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"]),
        world.crypto.generateKey({ name: "Ed25519" }, true, ["sign", "verify"]),
        world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"]),
    ]);
    const signingKey = await PublicKey.of(p256.publicKey, world.crypto);
    const rotationKeys = [signingKey, await PublicKey.of(ed25519.publicKey, world.crypto)];
    const operation: IdentityOperation = {
        subject: "space-1",
        previous: null,
        signingKeys: [signingKey],
        rotationKeys,
    };

    // verify the operation signed by each key at its priority
    const signed = await Promise.all(
        [p256, ed25519, stranger].map((pair) =>
            IdentityOperation.sign(operation, pair.privateKey, world.crypto),
        ),
    );
    expect([
        signed.map((each) => IdentityOperation.read(each).subject),
        await Promise.all(
            signed.map((each) => IdentityOperation.verify(each, rotationKeys, world.crypto)),
        ),
    ]).toEqual([
        ["space-1", "space-1", "space-1"],
        [0, 1, undefined],
    ]);
});

test("refuse reading a token that is no identity operation", async () => {
    expect(await refusal(() => IdentityOperation.read("eyJhbGciOiJub25lIn0.e30."))).toEqual([
        "BAD_REQUEST",
        "OPERATION_INVALID",
        "the operation is not an identity operation",
    ]);
});
