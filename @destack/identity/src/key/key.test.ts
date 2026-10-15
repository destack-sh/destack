import { expect, test } from "@destack/test";
import { KeyPair, PublicKey } from "./key.ts";

/** The signing algorithm of key pairs. */
const SIGNING = { name: "ECDSA", hash: "SHA-256" } as const;

test("read a generated key pair back from what a keychain keeps, signing as the generated one", async () => {
    // generate a pair and read it back from its kept private half
    const { pair, kept } = await KeyPair.generate();
    const read = await KeyPair.read(kept);

    // sign with the read private half and verify with the generated public half
    const message = new TextEncoder().encode("destack");
    const signature = await crypto.subtle.sign(SIGNING, read.privateKey, message);
    const verifier = await PublicKey.import(pair.publicKey);

    expect({
        publicKey: read.publicKey,
        isExtractable: read.privateKey.extractable,
        isVerified: await crypto.subtle.verify(SIGNING, verifier, signature, message),
    }).toEqual({ publicKey: pair.publicKey, isExtractable: false, isVerified: true });
});
