import { defineSchema, schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";

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

/** A public key as a JSON Web Key, verifying what its private half signs: identities', machines', clients' and installations' keys. */
export const PublicKey = Object.assign(defineSchema(publicKey), {
    /** Export a key pair's public half. */
    async of(key: CryptoKey): Promise<PublicKey> {
        const { kty, crv, x, y } = await crypto.subtle.exportKey("jwk", key);

        return PublicKey.parse(kty === "OKP" ? { kty, crv, x } : { kty, crv, x, y });
    },

    /** List the JWS algorithms a key verifies: ES256 for P-256, EdDSA and Ed25519 (RFC 9864) for Ed25519. */
    algorithms(key: PublicKey): [string, ...string[]] {
        return key.kty === "OKP" ? ["Ed25519", "EdDSA"] : ["ES256"];
    },

    /** Import a key for verifying, refusing a point off its curve. */
    async import(key: PublicKey): Promise<CryptoKey> {
        const algorithm =
            key.kty === "OKP" ? { name: "Ed25519" } : { name: "ECDSA", namedCurve: key.crv };
        try {
            return await crypto.subtle.importKey("jwk", key, algorithm, false, ["verify"]);
        } catch (error) {
            throw new IdentityError("INVALID_PUBLIC_KEY", "invalid public key", { cause: error });
        }
    },

    /** Compute a key's RFC 7638 thumbprint: the SHA-256 of its required members in lexicographic order. */
    async thumbprint(key: PublicKey): Promise<string> {
        // order the key type's required members by name
        const members = key.kty === "OKP" ? { crv: key.crv, kty: key.kty, x: key.x } : key;
        const ordered = Object.fromEntries(
            Object.entries(members).toSorted(([left], [right]) => (left < right ? -1 : 1)),
        );

        // hash their JSON with SHA-256
        const digest = await crypto.subtle.digest(
            "SHA-256",
            new TextEncoder().encode(JSON.stringify(ordered)),
        );

        return new Uint8Array(digest).toBase64({ alphabet: "base64url", omitPadding: true });
    },

    /** Publish signing keys as a JSON Web Key Set (RFC 7517 5), each named by its thumbprint with the algorithm it verifies. */
    async keySet(keys: readonly PublicKey[]): Promise<KeySet> {
        return {
            keys: await Promise.all(
                keys.map(async (key) => {
                    const kid = await PublicKey.thumbprint(key);

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

/** Published signing keys as a JSON Web Key Set (RFC 7517 5): each key with its algorithm, use and thumbprint. */
export const KeySet = defineSchema(
    schema.object({
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
    }),
);
/** Published signing keys as a JSON Web Key Set. */
export type KeySet = schema.Infer<typeof KeySet>;
