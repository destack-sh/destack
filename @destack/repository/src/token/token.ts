/** The JWS algorithm and Web Crypto signing parameters of each supported private key type. */
const ALGORITHMS = {
    "RSASSA-PKCS1-v1_5": { name: "RS256", parameters: { name: "RSASSA-PKCS1-v1_5" } },
    ECDSA: { name: "ES256", parameters: { name: "ECDSA", hash: "SHA-256" } },
} as const;

/** A compact JSON Web Token signed with Web Crypto. */
export class JsonWebToken {
    /** Sign claims with an RS256 or P-256 ES256 private key. */
    static async sign(claims: Readonly<Record<string, unknown>>, key: CryptoKey): Promise<string> {
        // select the algorithm the key signs with, refusing curves and hashes other than ES256's and RS256's
        const algorithm = ALGORITHMS[key.algorithm.name as keyof typeof ALGORITHMS];
        const parameters = key.algorithm as Partial<EcKeyAlgorithm & RsaHashedKeyAlgorithm>;
        if (algorithm === undefined) {
            throw new TypeError(`json web tokens are not signed with ${key.algorithm.name} keys`);
        } else if (
            (algorithm.name === "ES256" && parameters.namedCurve !== "P-256") ||
            (algorithm.name === "RS256" && parameters.hash?.name !== "SHA-256")
        ) {
            throw new TypeError(`${algorithm.name} signs with P-256 or SHA-256 keys only`);
        }

        // sign the encoded header and claims
        const header = JsonWebToken.#encode({ alg: algorithm.name, typ: "JWT" });
        const content = `${header}.${JsonWebToken.#encode(claims)}`;
        const signature = await crypto.subtle.sign(
            algorithm.parameters,
            key,
            new TextEncoder().encode(content),
        );

        return `${content}.${new Uint8Array(signature).toBase64({ alphabet: "base64url", omitPadding: true })}`;
    }

    /** Encode a JSON value as unpadded base64url. */
    static #encode(value: unknown): string {
        return new TextEncoder()
            .encode(JSON.stringify(value))
            .toBase64({ alphabet: "base64url", omitPadding: true });
    }
}
