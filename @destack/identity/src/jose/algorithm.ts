import { schema } from "@destack/schema";

/** The JWS algorithms keys sign and verify with (RFC 7518 3.1, RFC 8037 3.1, RFC 9864 2). */
export const JWS_ALGORITHMS = ["ES256", "ES384", "EdDSA", "Ed25519", "RS256", "HS256"] as const;

/** One JWS algorithm's keys and signatures: WebCrypto's parameters and the JWK's type and curve. */
interface AlgorithmParameters {
    /** The signature's WebCrypto algorithm and hash. */
    readonly signature: { readonly name: string; readonly hash?: string };
    /** The key's WebCrypto algorithm, curve and hash. */
    readonly key: { readonly name: string; readonly namedCurve?: string; readonly hash?: string };
    /** The JWK key type (RFC 7518 6.1, RFC 8037 2). */
    readonly kty: string;
    /** The JWK curve of an elliptic or Edwards key. */
    readonly crv?: string;
}

/** The keys and signatures of each algorithm. */
const PARAMETERS: Readonly<Record<JwsAlgorithm, AlgorithmParameters>> = {
    ES256: {
        signature: { name: "ECDSA", hash: "SHA-256" },
        key: { name: "ECDSA", namedCurve: "P-256" },
        kty: "EC",
        crv: "P-256",
    },
    ES384: {
        signature: { name: "ECDSA", hash: "SHA-384" },
        key: { name: "ECDSA", namedCurve: "P-384" },
        kty: "EC",
        crv: "P-384",
    },
    EdDSA: { signature: { name: "Ed25519" }, key: { name: "Ed25519" }, kty: "OKP", crv: "Ed25519" },
    Ed25519: {
        signature: { name: "Ed25519" },
        key: { name: "Ed25519" },
        kty: "OKP",
        crv: "Ed25519",
    },
    RS256: {
        signature: { name: "RSASSA-PKCS1-v1_5" },
        key: { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" },
        kty: "RSA",
    },
    HS256: { signature: { name: "HMAC" }, key: { name: "HMAC", hash: "SHA-256" }, kty: "oct" },
};

/** A JWS algorithm. */
export type JwsAlgorithm = (typeof JWS_ALGORITHMS)[number];

/** JWS algorithms, parsed by name. */
export const JwsAlgorithm = Object.assign(schema.enum(JWS_ALGORITHMS), {
    /** Read an algorithm's keys and signatures. */
    parameters(algorithm: JwsAlgorithm): AlgorithmParameters {
        return PARAMETERS[algorithm];
    },

    /** Read the first algorithm of a key type and curve, absent when none signs with it. */
    of(kty: string, crv: string | undefined): JwsAlgorithm | undefined {
        return JWS_ALGORITHMS.find(
            (algorithm) => PARAMETERS[algorithm].kty === kty && PARAMETERS[algorithm].crv === crv,
        );
    },
});
