import { PublicKey } from "../identity/identity.ts";

/** Generate a P-256 key pair, its public half as a JSON Web Key. */
export async function keyPair(): Promise<{
    readonly key: PublicKey;
    readonly privateKey: CryptoKey;
}> {
    const pair = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const { kty, crv, x, y } = await crypto.subtle.exportKey("jwk", pair.publicKey);

    return { key: PublicKey.parse({ kty, crv, x, y }), privateKey: pair.privateKey };
}
