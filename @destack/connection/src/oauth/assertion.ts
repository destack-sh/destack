import { present, Uuid } from "@destack/schema";
import { type Environment } from "@destack/environment";
import { Jwk, Jws, JwsAlgorithm, Jwt } from "@destack/identity";
import { ConnectionError } from "../error/error.ts";

/** How long an assertion stays valid: five minutes. */
const LIFETIME_SECONDS = 5 * 60;

/** The seconds an assertion's issue time dates back against clock drift, as GitHub advises. */
const DRIFT_SECONDS = 60;

/** The DER of a PKCS #8 RSA private key before its PKCS #1 key's length. */
const PKCS8_RSA_HEADER = Uint8Array.fromHex("020100300d06092a864886f70d0101010500");

/** JWT assertions a client or a service account signs for an authorization server (RFC 7523). */
export const Assertion = {
    /** Sign an assertion issued now, in Unix milliseconds. */
    async sign(
        claims: {
            readonly issuer: string;
            readonly subject?: string;
            readonly audience?: string;
            readonly key: string;
            readonly algorithm?: string;
            readonly scope?: string;
        },
        now: number,
        environment: Pick<Environment, "crypto" | "random">,
    ): Promise<string> {
        // import the key with its algorithm
        const { crypto, random } = environment;
        const { key, algorithm } = await Assertion.importKey(claims.key, claims.algorithm, crypto);

        // write the short-lived claims an authorization server checks (RFC 7523 3)
        const seconds = Math.floor(now / 1000);
        const signed = {
            ...(claims.scope === undefined ? {} : { scope: claims.scope }),
            iss: claims.issuer,
            ...(claims.subject === undefined ? {} : { sub: claims.subject }),
            ...(claims.audience === undefined ? {} : { aud: claims.audience }),
            iat: seconds - DRIFT_SECONDS,
            exp: seconds + LIFETIME_SECONDS,
            jti: Uuid.v4(random),
        };

        return Jwt.sign(signed, { alg: algorithm }, key, crypto);
    },

    /** Import a JWK's JSON or PKCS #1 or PKCS #8 PEM private key, with its signing algorithm. */
    async importKey(
        text: string,
        named: string | undefined,
        crypto: Environment["crypto"],
    ): Promise<{ readonly key: CryptoKey; readonly algorithm: JwsAlgorithm }> {
        // import a PEM with the given algorithm, wrapping a PKCS #1 RSA key as GitHub issues them
        const pem = text.trimStart();
        if (pem.startsWith("-----BEGIN PRIVATE KEY-----")) {
            const algorithm = Assertion.algorithm(named ?? "RS256");

            return { key: await Jws.importPkcs8(pem, algorithm, crypto), algorithm };
        } else if (pem.startsWith("-----BEGIN RSA PRIVATE KEY-----")) {
            const algorithm = Assertion.algorithm(named ?? "RS256");

            return { key: await Jws.importPkcs8(pkcs8(pem), algorithm, crypto), algorithm };
        }

        // import a JWK with the given algorithm, else its own or its key type's
        const jwk = Jwk.parse(JSON.parse(text));
        const algorithm = named === undefined ? Jwk.algorithm(jwk) : Assertion.algorithm(named);
        if (algorithm === undefined) {
            throw ConnectionError.of(
                "SIGNING_KEY_INVALID",
                `a ${jwk.kty} key has no signature algorithm`,
            );
        }
        const key = await Jws.importJwk(jwk, algorithm, crypto);
        if (key.type !== "private") {
            throw ConnectionError.of("SIGNING_KEY_INVALID", "the signing key is not a private key");
        }

        return { key, algorithm };
    },

    /** Read a signature algorithm's name, refusing one no key signs with. */
    algorithm(name: string): JwsAlgorithm {
        const algorithm = JwsAlgorithm.safeParse(name).data;
        if (algorithm === undefined) {
            throw ConnectionError.of(
                "SIGNING_KEY_INVALID",
                `the algorithm ${name} is not supported`,
            );
        }

        return algorithm;
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
