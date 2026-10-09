import { present, schema } from "@destack/schema";
import { importJWK, importPKCS8, SignJWT } from "jose";
import { ConnectionError } from "../error/error.ts";

/** How long an assertion stays valid, five minutes as RFC 7523 3 asks for short lifetimes and under GitHub's ten-minute limit for app JWTs. */
const LIFETIME_SECONDS = 5 * 60;

/** The seconds an assertion's issue time dates back against clock drift, a minute as GitHub advises for app JWTs. */
const DRIFT_SECONDS = 60;

/** The DER of a PKCS #8 RSA private key before its PKCS #1 key's length: the version and the rsaEncryption algorithm. */
const PKCS8_RSA_HEADER = Uint8Array.fromHex("020100300d06092a864886f70d0101010500");

/** The signature algorithm of each JWK key type and curve without an `alg`. */
const ALGORITHMS: Readonly<Record<string, string>> = {
    RSA: "RS256",
    "EC P-256": "ES256",
    "EC P-384": "ES384",
    "OKP Ed25519": "EdDSA",
};

/** A private key as a JWK (RFC 7517), read for its type, curve and algorithm. */
export const PrivateJwk = schema.looseObject({
    kty: schema.string().min(1),
    crv: schema.string().min(1).exactOptional(),
    alg: schema.string().min(1).exactOptional(),
});

/** JWT assertions a client or a service account signs for an authorization server (RFC 7523). */
export const Assertion = {
    /** Sign an assertion with a private key given as a JWK's JSON or a PKCS #1 or PKCS #8 PEM, the PEM signing with RS256 unless named, without a subject and audience for a GitHub app JWT. */
    async sign(claims: {
        readonly issuer: string;
        readonly subject?: string;
        readonly audience?: string;
        readonly key: string;
        readonly algorithm?: string;
        readonly scope?: string;
    }): Promise<string> {
        // import the key with its algorithm
        const { key, algorithm } = await Assertion.importKey(claims.key, claims.algorithm);

        // write the short-lived claims with a fresh identifier
        const now = Math.floor(Date.now() / 1000);
        const jwt = new SignJWT(claims.scope === undefined ? {} : { scope: claims.scope })
            .setProtectedHeader({ alg: algorithm })
            .setIssuer(claims.issuer)
            .setIssuedAt(now - DRIFT_SECONDS)
            .setExpirationTime(now + LIFETIME_SECONDS)
            .setJti(crypto.randomUUID());

        // set the subject and audience an authorization server checks (RFC 7523 3)
        if (claims.subject !== undefined) {
            jwt.setSubject(claims.subject);
        }
        if (claims.audience !== undefined) {
            jwt.setAudience(claims.audience);
        }

        return jwt.sign(key);
    },

    /** Import a private key given as a JWK's JSON or a PKCS #1 or PKCS #8 PEM, with the algorithm it signs with. */
    async importKey(
        text: string,
        named?: string,
    ): Promise<{ readonly key: CryptoKey; readonly algorithm: string }> {
        // import a PEM with the given algorithm, wrapping a PKCS #1 RSA key as GitHub issues them
        const pem = text.trimStart();
        if (pem.startsWith("-----BEGIN PRIVATE KEY-----")) {
            const algorithm = named ?? "RS256";

            return { key: await importPKCS8(pem, algorithm), algorithm };
        } else if (pem.startsWith("-----BEGIN RSA PRIVATE KEY-----")) {
            const algorithm = named ?? "RS256";

            return { key: await importPKCS8(pkcs8(pem), algorithm), algorithm };
        }

        // import a JWK with its algorithm or its key type's
        const jwk = PrivateJwk.parse(JSON.parse(text));
        const algorithm = named ?? jwk.alg ?? ALGORITHMS[`${jwk.kty} ${jwk.crv ?? ""}`.trim()];
        if (algorithm === undefined) {
            throw new ConnectionError(
                "INVALID_AUTHORIZATION",
                `a ${jwk.kty} key has no signature algorithm`,
            );
        }
        const key = await importJWK(jwk, algorithm);
        if (!(key instanceof CryptoKey)) {
            throw new ConnectionError("INVALID_AUTHORIZATION", "the signing key is no private key");
        }

        return { key, algorithm };
    },
};

/** Wrap a PKCS #1 RSA PEM into the PKCS #8 PEM WebCrypto imports. */
function pkcs8(pem: string): string {
    // read the PKCS #1 key's DER
    const body = /-----BEGIN RSA PRIVATE KEY-----([^-]+)-----END RSA PRIVATE KEY-----/u.exec(pem);
    const base64 = present(body?.[1], "the PKCS #1 key's body").replaceAll(/\s/gu, "");
    const pkcs1 = Uint8Array.fromBase64(base64);

    // wrap it as the private key octet string of a PKCS #8 sequence
    const octets = [0x04, ...length(pkcs1.length), ...pkcs1];
    const sequence = [...PKCS8_RSA_HEADER, ...octets];
    const der = Uint8Array.from([0x30, ...length(sequence.length), ...sequence]);
    const lines = present(der.toBase64().match(/.{1,64}/gu), "the key's base64 lines");

    return `-----BEGIN PRIVATE KEY-----\n${lines.join("\n")}\n-----END PRIVATE KEY-----\n`;
}

/** Encode a DER length. */
function length(value: number): number[] {
    // encode a short length in one byte
    if (value < 0x80) {
        return [value];
    }

    // encode a long length as its big-endian bytes after their count
    const bytes: number[] = [];
    for (let rest = value; rest > 0; rest >>= 8) {
        bytes.unshift(rest & 0xff);
    }

    return [0x80 | bytes.length, ...bytes];
}
