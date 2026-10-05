import { expect, test } from "@destack/test";
import { JsonWebToken } from "./token.ts";

/** Decode an unpadded base64url segment as JSON. */
function decode(segment: string): unknown {
    return JSON.parse(
        new TextDecoder().decode(Uint8Array.fromBase64(segment, { alphabet: "base64url" })),
    );
}

test("sign claims with ES256 and RS256 keys into tokens their public keys verify", async () => {
    // sign the same claims with a P-256 key and an RSA key
    const claims = { iss: "acme", sub: "destack", exp: 1790000060, iat: 1790000000 };
    const ecdsa = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, [
        "sign",
        "verify",
    ]);
    const rsa = await crypto.subtle.generateKey(
        {
            name: "RSASSA-PKCS1-v1_5",
            modulusLength: 2048,
            publicExponent: new Uint8Array([1, 0, 1]),
            hash: "SHA-256",
        },
        false,
        ["sign", "verify"],
    );
    const verified = async (
        token: string,
        key: CryptoKey,
        parameters: AlgorithmIdentifier | EcdsaParams,
    ) => {
        const [header, body, signature] = token.split(".");
        if (header === undefined || body === undefined || signature === undefined) {
            throw new TypeError("a compact jwt has three parts");
        }
        const isValid = await crypto.subtle.verify(
            parameters,
            key,
            Uint8Array.fromBase64(signature, { alphabet: "base64url" }),
            new TextEncoder().encode(`${header}.${body}`),
        );

        return [decode(header), decode(body), isValid];
    };

    // name each key's algorithm in the header and verify each signature
    expect([
        await verified(await JsonWebToken.sign(claims, ecdsa.privateKey), ecdsa.publicKey, {
            name: "ECDSA",
            hash: "SHA-256",
        }),
        await verified(await JsonWebToken.sign(claims, rsa.privateKey), rsa.publicKey, {
            name: "RSASSA-PKCS1-v1_5",
        }),
    ]).toEqual([
        [{ alg: "ES256", typ: "JWT" }, claims, true],
        [{ alg: "RS256", typ: "JWT" }, claims, true],
    ]);
});

test("refuse signing with keys of other algorithms", async () => {
    const key = await crypto.subtle.generateKey({ name: "HMAC", hash: "SHA-256" }, false, ["sign"]);

    await expect(JsonWebToken.sign({}, key)).rejects.toThrow(
        new TypeError("json web tokens are not signed with HMAC keys"),
    );
});
