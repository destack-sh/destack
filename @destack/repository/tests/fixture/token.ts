import { found, type JsonObject, schema } from "@destack/schema";

/** The Web Crypto verification parameters of each JWS algorithm the stand-ins accept. */
const ALGORITHMS: ReadonlyMap<string, AlgorithmIdentifier | EcdsaParams> = new Map([
    ["RS256", { name: "RSASSA-PKCS1-v1_5" }],
    ["ES256", { name: "ECDSA", hash: "SHA-256" }],
]);

/** The JWT header fields the stand-ins read. */
const Header = schema.looseObject({ alg: schema.string() });

/** The claims of a JWT. */
const Claims = schema.record(schema.string(), schema.json());

/** Verify a compact JWT's signature with a public key, returning its header algorithm and claims. */
export async function verifyToken(
    token: string,
    key: CryptoKey,
): Promise<{ readonly alg: string; readonly claims: JsonObject }> {
    // decode the header and claims
    const [header, claims, signature] = token.split(".");
    if (header === undefined || claims === undefined || signature === undefined) {
        throw new TypeError("a compact jwt has three parts");
    }
    const { alg } = Header.parse(decode(header));

    // require the key's signature over the header and claims
    const isValid = await crypto.subtle.verify(
        found(ALGORITHMS, alg),
        key,
        Uint8Array.fromBase64(signature, { alphabet: "base64url" }),
        new TextEncoder().encode(`${header}.${claims}`),
    );
    if (!isValid) {
        throw new Error(`invalid ${alg} signature`);
    }

    return { alg, claims: Claims.parse(decode(claims)) };
}

/** Decode a base64url JSON part of a compact JWT. */
function decode(part: string): unknown {
    return JSON.parse(
        new TextDecoder().decode(Uint8Array.fromBase64(part, { alphabet: "base64url" })),
    );
}
