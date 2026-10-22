import { defineTable, json, text } from "@destack/db";
import { Ciphertext, PublicKey } from "@destack/identity";
import { Instant, schema } from "@destack/schema";

/** A signing key an identity publishes. */
export const SigningKey = schema.object({
    /** The public half, as the identity publishes it. */
    publicKey: PublicKey,
    /** The private half as encrypted PKCS #8 bytes. */
    privateKey: Ciphertext,
    /** When the identity first published it. */
    publishedAt: Instant,
});
/** A signing key an identity publishes. */
export type SigningKey = schema.Infer<typeof SigningKey>;

/** The private keys and root secret of each identity this process stores. */
export const identityKey = defineTable("identity_key", {
    /** The space or universe the identity is, the keys' scope. */
    scope: text("scope").primaryKey(),
    /** The signing keys in the order the identity publishes them, newest first. */
    signingKeys: json("signing_keys", schema.array(SigningKey).min(1)).notNull(),
    /** The process's private rotation key of the identity, as encrypted PKCS #8 bytes. */
    rotation: text("rotation").notNull(),
    /** The public half of the process's rotation key. */
    rotationKey: json("rotation_key", PublicKey).notNull(),
    /** The identity's root secret its keys derive from, as 32 encrypted random bytes. */
    root: text("root").notNull(),
});
