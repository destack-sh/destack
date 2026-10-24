import type { Crypto } from "@destack/environment";
import { schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { Base64Url } from "./base64.ts";
import { JwsAlgorithm } from "./algorithm.ts";
import { Jwk } from "./jwk.ts";

/** The fewest modulus bits an RS256 key may have (RFC 7518 3.3). */
const RSA_MODULUS_BITS = 2048;

/** The fewest bits an HS256 secret may have, its hash's output (RFC 7518 3.2). */
const HMAC_SECRET_BITS = 256;

/** A PKCS #8 private key in PEM, its base64 body captured (RFC 7468 10). */
const PKCS8_PEM = /^-----BEGIN PRIVATE KEY-----([A-Za-z0-9+/=\s]+)-----END PRIVATE KEY-----$/u;

/** A JWS protected header (RFC 7515 4.1). */
export const JwsHeader = schema.looseObject({
    /** The signing algorithm. */
    alg: schema.string().min(1),
    /** The identifier of the signing key. */
    kid: schema.string().min(1).exactOptional(),
    /** The media type of the whole token. */
    typ: schema.string().min(1).exactOptional(),
    /** The media type of the payload. */
    cty: schema.string().min(1).exactOptional(),
});
/** A JWS protected header. */
export type JwsHeader = schema.Infer<typeof JwsHeader>;

/** A key a JWS signs or verifies with: a WebCrypto key, or an HMAC secret's bytes. */
export type JwsKey = CryptoKey | Uint8Array<ArrayBuffer>;

/** The keys that may have signed a JWS, read from its protected header. */
export type KeyLookup = (header: JwsHeader) => Promise<readonly JwsKey[]>;

/** What a verified JWS has: its protected header and its payload's bytes. */
export interface VerifiedJws {
    /** The protected header. */
    readonly header: JwsHeader;
    /** The payload's bytes. */
    readonly payload: Uint8Array<ArrayBuffer>;
}

/** Signed payloads in the compact serialization (RFC 7515 7.1). */
export const Jws = {
    /** Sign a payload under a protected header with a key. */
    async sign(
        payload: Uint8Array,
        header: JwsHeader & { readonly alg: JwsAlgorithm },
        key: JwsKey,
        crypto: Crypto,
    ): Promise<string> {
        // sign the encoded header and payload
        const input = `${Base64Url.encode(JSON.stringify(header))}.${Base64Url.encode(payload)}`;
        const signing = await signingKey(key, header.alg, crypto);
        const signature = await crypto.sign(
            JwsAlgorithm.parameters(header.alg).signature,
            signing,
            new TextEncoder().encode(input),
        );

        return `${input}.${Base64Url.encode(new Uint8Array(signature))}`;
    },

    /** Verify a JWS against its possible keys, refusing an algorithm outside the allowed ones. */
    async verify(
        compact: string,
        keys: JwsKey | KeyLookup,
        algorithms: readonly JwsAlgorithm[],
        crypto: Crypto,
    ): Promise<VerifiedJws> {
        // read the three segments and the header, refusing extensions it marks critical
        const [encodedHeader, encodedPayload, encodedSignature] = segments(compact);
        const header = parseHeader(encodedHeader);
        if ("crit" in header) {
            throw IdentityError.of("TOKEN_MALFORMED", "the token marks an extension critical");
        }

        // refuse an algorithm outside the allowed ones, such as none
        const algorithm = JwsAlgorithm.safeParse(header.alg).data;
        if (algorithm === undefined || !algorithms.includes(algorithm)) {
            throw IdentityError.of("ALGORITHM_REFUSED", `the algorithm ${header.alg} is refused`);
        }

        // filter the candidate keys of the algorithm, refusing a key of another
        const candidates = typeof keys === "function" ? await keys(header) : [keys];
        const fitting = candidates.filter((key) => fits(key, algorithm));
        if (fitting.length === 0) {
            throw IdentityError.of(
                "ALGORITHM_REFUSED",
                `no key verifies the algorithm ${algorithm}`,
            );
        }

        // accept the signature of any candidate
        const signature = Base64Url.decode(encodedSignature);
        const input = new TextEncoder().encode(`${encodedHeader}.${encodedPayload}`);
        for (const key of fitting) {
            const verifying =
                key instanceof Uint8Array
                    ? await importSecret(key, algorithm, "verify", crypto)
                    : key;
            if (
                await crypto.verify(
                    JwsAlgorithm.parameters(algorithm).signature,
                    verifying,
                    signature,
                    input,
                )
            ) {
                return { header, payload: Base64Url.decode(encodedPayload) };
            }
        }

        // refuse a signature no candidate verifies
        throw IdentityError.of("SIGNATURE_INVALID", "the token's signature is invalid");
    },

    /** Read a JWS's protected header without verifying it. */
    header(compact: string): JwsHeader {
        const [encodedHeader] = segments(compact);

        return parseHeader(encodedHeader);
    },

    /** Read a JWS's payload without verifying it. */
    payload(compact: string): Uint8Array<ArrayBuffer> {
        const [, encodedPayload] = segments(compact);

        return Base64Url.decode(encodedPayload);
    },

    /** Import a JSON Web Key for an algorithm: private and secret keys sign, public keys verify. */
    async importJwk(jwk: Jwk, algorithm: JwsAlgorithm, crypto: Crypto): Promise<CryptoKey> {
        // refuse a key of another type, curve or algorithm
        if (!Jwk.fits(jwk, algorithm)) {
            throw IdentityError.of("KEY_INVALID", `the key is not an ${algorithm} key`);
        }

        // sign and verify with a secret, sign with a private key, and verify with a public one
        const usages: ("sign" | "verify")[] =
            jwk.kty === "oct" ? ["sign", "verify"] : "d" in jwk ? ["sign"] : ["verify"];

        return checkKey(
            crypto.importKey("jwk", jwk, JwsAlgorithm.parameters(algorithm).key, false, usages),
            algorithm,
        );
    },

    /** Import a PKCS #8 PEM private key that signs with an algorithm. */
    async importPkcs8(pem: string, algorithm: JwsAlgorithm, crypto: Crypto): Promise<CryptoKey> {
        // read the PEM's base64 body, refusing another label
        const body = PKCS8_PEM.exec(pem.trim())?.[1];
        if (body === undefined) {
            throw IdentityError.of("KEY_INVALID", "the key is not a PKCS #8 PEM private key");
        }

        // decode its DER for signing
        let der: Uint8Array<ArrayBuffer>;
        try {
            der = Uint8Array.fromBase64(body.replaceAll(/\s/gu, ""));
        } catch (error) {
            throw IdentityError.of("KEY_INVALID", "the PEM body is invalid base64", {
                cause: error,
            });
        }

        return checkKey(
            crypto.importKey("pkcs8", der, JwsAlgorithm.parameters(algorithm).key, false, ["sign"]),
            algorithm,
        );
    },
};

/** Read the key a JWS signs with, refusing a key of another algorithm. */
async function signingKey(
    key: JwsKey,
    algorithm: JwsAlgorithm,
    crypto: Crypto,
): Promise<CryptoKey> {
    // refuse a key of another algorithm
    if (!fits(key, algorithm)) {
        throw IdentityError.of("KEY_INVALID", `the key does not sign ${algorithm}`);
    }

    return key instanceof Uint8Array ? importSecret(key, algorithm, "sign", crypto) : key;
}

/** Import an HMAC secret's bytes for one use. */
function importSecret(
    secret: Uint8Array<ArrayBuffer>,
    algorithm: JwsAlgorithm,
    usage: "sign" | "verify",
    crypto: Crypto,
): Promise<CryptoKey> {
    return checkKey(
        crypto.importKey("raw", secret, JwsAlgorithm.parameters(algorithm).key, false, [usage]),
        algorithm,
    );
}

/** Await an imported key, refusing an invalid key, a short RSA modulus and a short HMAC secret. */
async function checkKey(imported: Promise<CryptoKey>, algorithm: JwsAlgorithm): Promise<CryptoKey> {
    // refuse a key WebCrypto refuses
    let key: CryptoKey;
    try {
        key = await imported;
    } catch (error) {
        throw IdentityError.of("KEY_INVALID", `the ${algorithm} key is invalid`, { cause: error });
    }

    // refuse a short RSA modulus or HMAC secret
    const keyAlgorithm: CryptoKey["algorithm"] & {
        readonly modulusLength?: number;
        readonly length?: number;
    } = key.algorithm;
    const isHmac = keyAlgorithm.name === "HMAC";
    const bits = isHmac ? keyAlgorithm.length : keyAlgorithm.modulusLength;
    const least = isHmac ? HMAC_SECRET_BITS : RSA_MODULUS_BITS;
    if (bits !== undefined && bits < least) {
        throw IdentityError.of("KEY_INVALID", `the ${algorithm} key has ${bits} bits`);
    }

    return key;
}

/** Decode and parse a JWS protected header, refusing an invalid one. */
function parseHeader(encodedHeader: string): JwsHeader {
    const parsed = JwsHeader.safeParse(Base64Url.object(encodedHeader));
    if (!parsed.success) {
        throw IdentityError.of("TOKEN_MALFORMED", "the token has an invalid header");
    }

    return parsed.data;
}

/** Split a compact JWS into its three segments. */
function segments(compact: string): readonly [string, string, string] {
    // split at the dots, refusing any count but three
    const split = compact.split(".");
    const [header, payload, signature] = split;
    if (
        split.length !== 3 ||
        header === undefined ||
        payload === undefined ||
        signature === undefined
    ) {
        throw IdentityError.of("TOKEN_MALFORMED", "the token is not a compact JWS");
    }

    return [header, payload, signature];
}

/** Report whether a key belongs to an algorithm: its curve, hash or kind. */
function fits(key: JwsKey, algorithm: JwsAlgorithm): boolean {
    // let an HMAC secret's bytes verify HMAC only
    const expected = JwsAlgorithm.parameters(algorithm).key;
    if (key instanceof Uint8Array) {
        return expected.name === "HMAC";
    }

    // match the key's algorithm name, curve and hash
    const actual: CryptoKey["algorithm"] & {
        readonly namedCurve?: string;
        readonly hash?: { readonly name: string };
    } = key.algorithm;
    const curve = expected.namedCurve;
    const hash = expected.hash;

    return (
        actual.name === expected.name &&
        (curve === undefined || actual.namedCurve === curve) &&
        (hash === undefined || actual.hash?.name === hash)
    );
}
