import { defineSchema, Digest, schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { PublicKey } from "../key/key.ts";
import { CompactSign, compactVerify, decodeJwt, decodeProtectedHeader, errors } from "jose";

/** The type a signed identity operation's header names. */
const OPERATION_TYPE = "identity-operation+jws";

/** The most rotation keys an identity lists. */
const MAX_ROTATION_KEYS = 5;

/** The most signing keys an identity publishes: the next one, the active one, and the one before it. */
export const MAX_SIGNING_KEYS = 3;

/** A space's or the universe's identity: the keys verifying what it signs, the keys that may change it, and the operation it comes from. */
export const Identity = defineSchema(
    schema.object({
        /** The keys verifying what the identity signs, newest first: a next key published ahead of signing, the active one, and earlier ones kept while tokens they signed live. */
        signingKeys: schema.array(PublicKey).min(1).max(MAX_SIGNING_KEYS),
        /** The keys that may sign the identity's next operation, highest priority first. */
        rotationKeys: schema.array(PublicKey).min(1).max(MAX_ROTATION_KEYS),
        /** The digest of the operation it comes from, which the next operation names. */
        digest: Digest,
    }),
);
/** A space's or the universe's identity. */
export type Identity = schema.Infer<typeof Identity>;

/** A change of an identity: its next signing and rotation keys, after the operation it follows. */
const identityOperation = Identity.omit({ digest: true }).extend({
    /** The space or universe the identity is. */
    subject: schema.string().min(1),
    /** The digest of the operation it follows, null for the identity's first. */
    previous: Digest.nullable(),
});

/** A change of an identity, signed by a rotation key of the identity before it, as a directory applies it. */
export const IdentityOperation = Object.assign(defineSchema(identityOperation), {
    /** Sign an operation with a rotation key's private half, as a compact JWS in the key's algorithm. */
    async sign(operation: IdentityOperation, rotationKey: CryptoKey): Promise<string> {
        const alg = rotationKey.algorithm.name === "Ed25519" ? "Ed25519" : "ES256";

        return new CompactSign(new TextEncoder().encode(JSON.stringify(operation)))
            .setProtectedHeader({ alg, typ: OPERATION_TYPE })
            .sign(rotationKey);
    },

    /** Read a signed operation without verifying its signature, refusing a malformed one. */
    read(signed: string): IdentityOperation {
        const claims = identityOperation.safeParse(decode(signed));
        if (!claims.success) {
            throw new IdentityError("INVALID_OPERATION", "the operation is no identity operation");
        }

        return claims.data;
    },

    /** Verify a signed operation against rotation keys, answering the priority of the key that signed it, absent when none did. */
    async verify(signed: string, rotationKeys: readonly PublicKey[]): Promise<number | undefined> {
        for (const [priority, key] of rotationKeys.entries()) {
            // try the next key when this one refuses the signature
            try {
                await compactVerify(signed, await PublicKey.import(key), {
                    algorithms: PublicKey.algorithms(key),
                });

                return priority;
            } catch (error) {
                if (!(error instanceof errors.JOSEError)) {
                    throw error;
                }
            }
        }

        return undefined;
    },

    /** Digest a signed operation, as the operation after it names it. */
    digest(signed: string): Promise<Digest> {
        return Digest.of(signed);
    },
});
/** A change of an identity. */
export type IdentityOperation = schema.Infer<typeof identityOperation>;

/** Decode the payload of an operation whose header names its type, absent for anything else. */
function decode(operation: string): unknown {
    try {
        return decodeProtectedHeader(operation).typ === OPERATION_TYPE
            ? decodeJwt(operation)
            : undefined;
    } catch (error) {
        if (!(error instanceof errors.JOSEError || error instanceof TypeError)) {
            throw error;
        }

        return undefined;
    }
}
