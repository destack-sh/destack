import type { Crypto } from "@destack/environment";
import { schema } from "@destack/schema";
import { Jwk } from "../jose/jwk.ts";
import type { JwsAlgorithm } from "../jose/algorithm.ts";
import { IdentityError } from "../error/error.ts";

/** The SEC 1 prefix of an uncompressed curve point (SEC 1 2.3.3). */
const UNCOMPRESSED_POINT = 0x04;

/** A base64url coordinate of a 256-bit curve point, 32 bytes unpadded. */
const COORDINATE = /^[A-Za-z0-9_-]{43}$/u;

/** An ECDSA P-256 public key as a JSON Web Key. */
const curveKey = schema
    .object({
        /** The key type of elliptic curves. */
        kty: schema.literal("EC"),
        /** The curve. */
        crv: schema.literal("P-256"),
        /** The x coordinate, base64url encoded. */
        x: schema.string().regex(COORDINATE),
        /** The y coordinate, base64url encoded. */
        y: schema.string().regex(COORDINATE),
    })
    .strict();

/** An Ed25519 public key as a JSON Web Key (RFC 8037). */
const edwardsKey = schema
    .object({
        /** The key type of Edwards curves. */
        kty: schema.literal("OKP"),
        /** The curve. */
        crv: schema.literal("Ed25519"),
        /** The public key, base64url encoded. */
        x: schema.string().regex(COORDINATE),
    })
    .strict();

/** A public key as a JSON Web Key: ECDSA over P-256, or Ed25519. */
const publicKey = schema.discriminatedUnion("kty", [curveKey, edwardsKey]);

/** What a key set adds to each published key beside its algorithm: its use and thumbprint. */
const published = {
    /** The key's use, signatures. */
    use: schema.literal("sig"),
    /** The key's RFC 7638 thumbprint, which tokens name in their header. */
    kid: schema.string().min(1),
};

/** A public key as a JSON Web Key, verifying what its private half signs. */
export const PublicKey = Object.assign(publicKey, {
    /** Export a key pair's public half. */
    async of(key: CryptoKey, crypto: Crypto): Promise<PublicKey> {
        const { kty, crv, x, y } = await crypto.exportKey("jwk", key);

        return PublicKey.parse(kty === "OKP" ? { kty, crv, x } : { kty, crv, x, y });
    },

    /** List the JWS algorithms a key verifies. */
    algorithms(key: PublicKey): readonly JwsAlgorithm[] {
        return key.kty === "OKP" ? ["Ed25519", "EdDSA"] : ["ES256"];
    },

    /** Import a key for verifying, refusing a point off its curve. */
    async import(key: PublicKey, crypto: Crypto): Promise<CryptoKey> {
        const algorithm =
            key.kty === "OKP" ? { name: "Ed25519" } : { name: "ECDSA", namedCurve: key.crv };
        try {
            return await crypto.importKey("jwk", key, algorithm, false, ["verify"]);
        } catch (error) {
            throw IdentityError.of("KEY_INVALID", "invalid public key", { cause: error });
        }
    },

    /** Write a key in WebCrypto's raw form: an uncompressed point or an Ed25519 key's bytes. */
    raw(key: PublicKey): Uint8Array<ArrayBuffer> {
        // write the Edwards key's bytes, or the curve point's prefix and coordinates
        const x = Uint8Array.fromBase64(key.x, { alphabet: "base64url" });
        if (key.kty === "OKP") {
            return x;
        }
        const y = Uint8Array.fromBase64(key.y, { alphabet: "base64url" });
        const point = new Uint8Array(1 + x.length + y.length);
        point.set([UNCOMPRESSED_POINT], 0);
        point.set(x, 1);
        point.set(y, 1 + x.length);

        return point;
    },

    /** Compute a key's RFC 7638 thumbprint. */
    thumbprint(key: PublicKey, crypto: Crypto): Promise<string> {
        return Jwk.thumbprint(key, crypto);
    },

    /** Publish signing keys as a JSON Web Key Set (RFC 7517 5). */
    async keySet(keys: readonly PublicKey[], crypto: Crypto): Promise<KeySet> {
        return {
            keys: await Promise.all(
                keys.map(async (key) => {
                    const kid = await PublicKey.thumbprint(key, crypto);

                    return key.kty === "OKP"
                        ? { ...key, alg: "Ed25519" as const, use: "sig" as const, kid }
                        : { ...key, alg: "ES256" as const, use: "sig" as const, kid };
                }),
            ),
        };
    },
});
/** A public key as a JSON Web Key. */
export type PublicKey = schema.Infer<typeof publicKey>;

/** The signing algorithm of key pairs: ECDSA over P-256 (ES256). */
const SIGNING = { name: "ECDSA", namedCurve: "P-256" } as const;

/** A P-256 private key as a JSON Web Key, as a keychain stores it. */
export const PrivateKey = schema.looseObject({
    /** The key type of elliptic curves. */
    kty: schema.literal("EC"),
    /** The curve. */
    crv: schema.literal("P-256"),
    /** The x coordinate, base64url encoded. */
    x: schema.string().regex(COORDINATE),
    /** The y coordinate, base64url encoded. */
    y: schema.string().regex(COORDINATE),
    /** The private scalar, base64url encoded, under its name in RFC 7518 6.2.2.1. */
    ["d"]: schema.string().regex(COORDINATE),
});

/** A P-256 private key as a JSON Web Key. */
export type PrivateKey = schema.Infer<typeof PrivateKey>;

/** A P-256 key pair: its public half, and its private half that signs. */
export interface KeyPair {
    /** The public half. */
    readonly publicKey: PublicKey;
    /** The private half, non-extractable, signing proofs and assertions. */
    readonly privateKey: CryptoKey;
}

/** Key pairs as keychains store them: their private half as a JSON Web Key. */
export const KeyPair = {
    /** Generate a key pair, with the private half a keychain stores. */
    async generate(crypto: Crypto): Promise<{ readonly pair: KeyPair; readonly secret: string }> {
        const generated = await crypto.generateKey(SIGNING, true, ["sign", "verify"]);
        const secret = JSON.stringify(await crypto.exportKey("jwk", generated.privateKey));

        return { pair: await KeyPair.read(secret, crypto), secret };
    },

    /** Read the key pair a keychain stores. */
    read(secret: string, crypto: Crypto): Promise<KeyPair> {
        return KeyPair.import(PrivateKey.parse(JSON.parse(secret)), crypto);
    },

    /** Import a private key for signing, with its public half from its coordinates. */
    async import(key: PrivateKey, crypto: Crypto): Promise<KeyPair> {
        const { kty, crv, x, y } = key;

        return {
            publicKey: PublicKey.parse({ kty, crv, x, y }),
            privateKey: await crypto.importKey("jwk", key, SIGNING, false, ["sign"]),
        };
    },
};

/** Published signing keys as a JSON Web Key Set (RFC 7517 5). */
export const KeySet = schema.object({
    /** The keys, newest first. */
    keys: schema.array(
        schema.discriminatedUnion("kty", [
            curveKey.extend({
                ...published,
                /** The JWS algorithm the key verifies. */
                alg: schema.literal("ES256"),
            }),
            edwardsKey.extend({
                ...published,
                /** The JWS algorithm the key verifies (RFC 9864). */
                alg: schema.literal("Ed25519"),
            }),
        ]),
    ),
});
/** Published signing keys as a JSON Web Key Set. */
export type KeySet = schema.Infer<typeof KeySet>;
