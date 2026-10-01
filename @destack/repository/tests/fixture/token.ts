/** The Web Crypto verification parameters of each JWS algorithm the stand-ins accept. */
const ALGORITHMS: Readonly<Record<string, AlgorithmIdentifier | EcdsaParams>> = {
    RS256: { name: "RSASSA-PKCS1-v1_5" },
    ES256: { name: "ECDSA", hash: "SHA-256" },
};

/** Verify a compact JWT's signature with a public key, returning its header algorithm and claims. */
export async function verifyToken(
    token: string,
    key: CryptoKey,
): Promise<{ readonly alg: string; readonly claims: Record<string, unknown> }> {
    // decode the header and claims
    const [header, claims, signature] = token.split(".") as [string, string, string];
    const decode = (part: string) =>
        JSON.parse(
            new TextDecoder().decode(Uint8Array.fromBase64(part, { alphabet: "base64url" })),
        );
    const { alg } = decode(header) as { alg: string };

    // require the key's signature over the header and claims
    const isValid = await crypto.subtle.verify(
        ALGORITHMS[alg]!,
        key,
        Uint8Array.fromBase64(signature, { alphabet: "base64url" }),
        new TextEncoder().encode(`${header}.${claims}`),
    );
    if (!isValid) {
        throw new Error(`invalid ${alg} signature`);
    }

    return { alg, claims: decode(claims) as Record<string, unknown> };
}
