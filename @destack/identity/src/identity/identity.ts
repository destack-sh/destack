import type { Crypto } from "@destack/environment";
import { Digest, schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { PublicKey } from "../key/key.ts";
import { Base64Url } from "../jose/base64.ts";
import { Jws } from "../jose/jws.ts";

/** The type a signed identity operation's header names. */
const OPERATION_TYPE = "identity-operation+jws";

/** The JWS algorithm of each WebCrypto rotation key algorithm. */
const ROTATION_ALGORITHMS: ReadonlyMap<string, "Ed25519" | "ES256"> = new Map([
    ["Ed25519", "Ed25519"],
    ["ECDSA", "ES256"],
]);

/** The most rotation keys an identity lists. */
const MAX_ROTATION_KEYS = 5;

/** The most signing keys an identity publishes: the next, the active and the previous one. */
export const MAX_SIGNING_KEYS = 3;

/** A space's or the universe's identity. */
export const Identity = schema.object({
    /** The keys that verify what the identity signs, newest first. */
    signingKeys: schema.array(PublicKey).min(1).max(MAX_SIGNING_KEYS),
    /** The keys that may sign the identity's next operation, highest priority first. */
    rotationKeys: schema.array(PublicKey).min(1).max(MAX_ROTATION_KEYS),
    /** The digest of the operation it comes from, which the next operation names. */
    digest: Digest,
});
/** A space's or the universe's identity. */
export type Identity = schema.Infer<typeof Identity>;

/** A change of an identity: its next signing and rotation keys, after the operation it follows. */
const identityOperation = Identity.omit({ digest: true }).extend({
    /** The space or universe the identity is. */
    subject: schema.string().min(1),
    /** The digest of the operation it follows, null for the identity's first. */
    previous: Digest.nullable(),
});

/** A change of an identity, as a directory applies it. */
export const IdentityOperation = Object.assign(identityOperation, {
    /** Sign an operation with a rotation key's private half. */
    sign(operation: IdentityOperation, rotationKey: CryptoKey, crypto: Crypto): Promise<string> {
        // sign with the rotation key's algorithm, refusing another curve
        const alg = ROTATION_ALGORITHMS.get(rotationKey.algorithm.name);
        if (alg === undefined) {
            throw IdentityError.of(
                "KEY_INVALID",
                "the rotation key is not an Ed25519 or P-256 key",
            );
        }
        const payload = new TextEncoder().encode(JSON.stringify(operation));

        return Jws.sign(payload, { alg, typ: OPERATION_TYPE }, rotationKey, crypto);
    },

    /** Read a signed operation without verifying its signature, refusing a malformed one. */
    read(signed: string): IdentityOperation {
        const claims = identityOperation.safeParse(decode(signed));
        if (!claims.success) {
            throw IdentityError.of(
                "OPERATION_INVALID",
                "the operation is not an identity operation",
            );
        }

        return claims.data;
    },

    /** Verify a signed operation against rotation keys. */
    async verify(
        signed: string,
        rotationKeys: readonly PublicKey[],
        crypto: Crypto,
    ): Promise<number | undefined> {
        for (const [priority, key] of rotationKeys.entries()) {
            // try the next key when this one refuses the signature
            try {
                const imported = await PublicKey.import(key, crypto);
                await Jws.verify(signed, imported, PublicKey.algorithms(key), crypto);

                return priority;
            } catch (error) {
                if (!IdentityError.is(error)) {
                    throw error;
                }
            }
        }

        return undefined;
    },

    /** Digest a signed operation, as the next operation refers to it. */
    digest(signed: string, crypto: Crypto): Promise<Digest> {
        return Digest.of(signed, crypto);
    },
});
/** A change of an identity. */
export type IdentityOperation = schema.Infer<typeof identityOperation>;

/** Decode the payload of an operation whose header states its type, absent for anything else. */
function decode(operation: string): unknown {
    try {
        return Jws.header(operation).typ === OPERATION_TYPE
            ? Base64Url.json(Jws.payload(operation))
            : undefined;
    } catch (error) {
        if (!IdentityError.is(error, "TOKEN_MALFORMED")) {
            throw error;
        }

        return undefined;
    }
}
