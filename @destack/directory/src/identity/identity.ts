import { boolean, defineTable, index, integer, text } from "@destack/db";
import { defineSchema, Digest, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { CompactSign, compactVerify, decodeProtectedHeader, errors, importJWK } from "jose";

/** The type a signed identity operation's header names. */
const OPERATION_TYPE = "identity-operation+jws";

/** The most rotation keys an identity lists. */
const MAX_ROTATION_KEYS = 5;

/** How long a higher-priority rotation key may nullify operations a lower one signed: 72 hours. */
export const RECOVERY_MILLISECONDS = 72 * 60 * 60 * 1000;

/** A P-256 public key as a JSON Web Key, verifying ES256 signatures. */
export const PublicKey = defineSchema(
    schema.object({
        /** The key type. */
        kty: schema.literal("EC"),
        /** The curve. */
        crv: schema.literal("P-256"),
        /** The x coordinate, base64url encoded. */
        x: schema.string().min(1),
        /** The y coordinate, base64url encoded. */
        y: schema.string().min(1),
    }),
);
/** A P-256 public key as a JSON Web Key. */
export type PublicKey = schema.Infer<typeof PublicKey>;

/** A space's identity: the key verifying what it signs, the keys that may change it, and the operation it comes from. */
export const Identity = defineSchema(
    schema.object({
        /** The key verifying what the space signs. */
        signingKey: PublicKey,
        /** The keys that may sign the identity's next operation, highest priority first. */
        rotationKeys: schema.array(PublicKey).min(1).max(MAX_ROTATION_KEYS),
        /** The digest of the operation it comes from, which the next operation names. */
        digest: Digest,
    }),
);
/** A space's identity. */
export type Identity = schema.Infer<typeof Identity>;

/** The claims of an identity operation: the space's next identity, after the operation it follows. */
const OperationClaims = Identity.omit({ digest: true }).extend({
    /** The space. */
    space: schema.string().min(1),
    /** The digest of the operation it follows, null for the space's first. */
    previous: Digest.nullable(),
});
/** The claims of an identity operation. */
export type OperationClaims = schema.Infer<typeof OperationClaims>;

/** The log of each space's identity operations, each signed by a rotation key of the identity before it. */
export const identityOperationTable = defineTable(
    "identity_operation",
    {
        /** The space. */
        space: text("space").primaryKey(),
        /** The operation's position in the space's log, from zero. */
        sequence: integer("sequence").primaryKey(),
        /** The universe, where every operation lives. */
        scope: text("scope").notNull(),
        /** The digest of the signed operation. */
        digest: text("digest").notNull(),
        /** The signed operation, a compact JSON Web Signature. */
        operation: text("operation").notNull(),
        /** The priority of the rotation key that signed it among the keys of the identity it follows. */
        priority: integer("priority").notNull(),
        /** When the directory applied it, in UTC epoch milliseconds. */
        appliedAt: integer("applied_at").notNull(),
        /** Whether a higher-priority rotation key nullified it. */
        isNullified: boolean("is_nullified").notNull(),
    },
    {
        log: {},
        constraints: (operation) => [index("identity_operation_digest").on(operation.digest)],
    },
);

/** Signed changes of a space's identity, as a directory applies them. */
export const IdentityOperation = {
    /** Sign an operation with a rotation key's private half. */
    async sign(claims: OperationClaims, rotationKey: CryptoKey): Promise<string> {
        return new CompactSign(new TextEncoder().encode(JSON.stringify(claims)))
            .setProtectedHeader({ alg: "ES256", typ: OPERATION_TYPE })
            .sign(rotationKey);
    },

    /** Read an operation's claims without verifying its signature, refusing a malformed one. */
    claims(operation: string): OperationClaims {
        const claims = OperationClaims.safeParse(decode(operation));
        if (!claims.success) {
            throw new ServiceError("BAD_REQUEST", {
                message: "the operation is no identity operation",
            });
        }

        return claims.data;
    },

    /** Find the priority of the rotation key that signed an operation, absent when none did. */
    async priority(
        operation: string,
        rotationKeys: readonly PublicKey[],
    ): Promise<number | undefined> {
        for (const [priority, key] of rotationKeys.entries()) {
            // try the next key when this one refuses the signature
            try {
                await compactVerify(operation, await importJWK(key, "ES256"), {
                    algorithms: ["ES256"],
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
    digest(operation: string): Promise<Digest> {
        return Digest.of(operation);
    },
};

/** Decode the payload of an operation whose header names its type, absent for anything else. */
function decode(operation: string): unknown {
    try {
        // require the operation type
        const [, payload] = operation.split(".");
        if (decodeProtectedHeader(operation).typ !== OPERATION_TYPE || payload === undefined) {
            return undefined;
        }

        // decode the JSON payload
        const bytes = Uint8Array.fromBase64(payload, { alphabet: "base64url" });

        return JSON.parse(new TextDecoder().decode(bytes));
    } catch (error) {
        if (
            !(
                error instanceof SyntaxError ||
                error instanceof TypeError ||
                error instanceof errors.JOSEError
            )
        ) {
            throw error;
        }

        return undefined;
    }
}
