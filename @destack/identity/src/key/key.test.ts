import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { KeyPair, PublicKey } from "./key.ts";

/** The signing algorithm of key pairs. */
const SIGNING = { name: "ECDSA", hash: "SHA-256" } as const;

test("read a generated key pair back from what a keychain stores, signing as the generated one", async () => {
    // generate a pair and read it back from its stored private half
    const world = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 24) }).environment("test");
    const { pair, secret } = await KeyPair.generate(world.crypto);
    const read = await KeyPair.read(secret, world.crypto);

    // sign with the read private half and verify with the generated public half
    const message = new TextEncoder().encode("destack");
    const signature = await world.crypto.sign(SIGNING, read.privateKey, message);
    const verifier = await PublicKey.import(pair.publicKey, world.crypto);

    expect({
        publicKey: read.publicKey,
        isExtractable: read.privateKey.extractable,
        isVerified: await world.crypto.verify(SIGNING, verifier, signature, message),
    }).toEqual({ publicKey: pair.publicKey, isExtractable: false, isVerified: true });
});
