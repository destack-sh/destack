import { type JsonObject, schema } from "@destack/schema";

/** The JWS algorithm and Web Crypto signing parameters of each supported private key type. */
const ALGORITHMS = {
    "RSASSA-PKCS1-v1_5": { name: "RS256", parameters: { name: "RSASSA-PKCS1-v1_5" } },
    ECDSA: { name: "ES256", parameters: { name: "ECDSA", hash: "SHA-256" } },
} as const;

/** The Web Crypto algorithm of a key: its name, an EC key's curve and an RSA key's hash. */
const KeyAlgorithm = schema.looseObject({
    /** The algorithm name, such as ECDSA. */
    name: schema.string(),
    /** The curve of an EC key. */
    namedCurve: schema.string().exactOptional(),
    /** The hash of an RSA key. */
    hash: schema.looseObject({ name: schema.string() }).exactOptional(),
});

/** A compact JSON Web Token signed with Web Crypto. */
export const JsonWebToken = {
    /** Sign claims with an RS256 or P-256 ES256 private key. */
    async sign(claims: JsonObject, key: CryptoKey): Promise<string> {
        // select the algorithm the key signs with, refusing curves and hashes other than ES256's and RS256's
        const parameters = KeyAlgorithm.parse(key.algorithm);
        const name = parameters.name;
        if (name !== "ECDSA" && name !== "RSASSA-PKCS1-v1_5") {
            throw new TypeError(`json web tokens are not signed with ${name} keys`);
        }
        const algorithm = ALGORITHMS[name];
        if (
            (algorithm.name === "ES256" && parameters.namedCurve !== "P-256") ||
            (algorithm.name === "RS256" && parameters.hash?.name !== "SHA-256")
        ) {
            throw new TypeError(`${algorithm.name} signs with P-256 or SHA-256 keys only`);
        }

        // sign the encoded header and claims
        const header = encode({ alg: algorithm.name, typ: "JWT" });
        const content = `${header}.${encode(claims)}`;
        const signature = await crypto.subtle.sign(
            algorithm.parameters,
            key,
            new TextEncoder().encode(content),
        );

        return `${content}.${new Uint8Array(signature).toBase64({ alphabet: "base64url", omitPadding: true })}`;
    },
};

/** Encode a JSON value as unpadded base64url. */
function encode(value: unknown): string {
    return new TextEncoder()
        .encode(JSON.stringify(value))
        .toBase64({ alphabet: "base64url", omitPadding: true });
}
