import { PublicKey } from "@destack/identity";

/** Generate a P-256 key pair, its public half as a JSON Web Key. */
export async function keyPair(): Promise<{
    readonly key: PublicKey;
    readonly privateKey: CryptoKey;
}> {
    const pair = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);

    return { key: await PublicKey.of(pair.publicKey), privateKey: pair.privateKey };
}
