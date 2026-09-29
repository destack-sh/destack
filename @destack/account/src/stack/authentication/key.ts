import { identifier, index, integer, type Select, defineTable, text } from "@destack/db";

/** A rotating authentication signing key. */
export const signingKey = defineTable(
    "signing_key",
    {
        /** The key identifier published in the JWKS document. */
        id: identifier("id", "signing-key").primaryKey(),
        /** The serialized public JWK. */
        publicKey: text("public_key").notNull(),
        /** The encrypted private JWK. */
        privateKey: text("private_key").notNull(),
        /** The creation time in epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The key expiry in epoch milliseconds. */
        expiresAt: integer("expires_at"),
        /** The JOSE signing algorithm. */
        alg: text("alg"),
        /** The elliptic curve name. */
        crv: text("crv"),
    },
    {
        tier: "global",
        constraints: (key) => [index("signing_key_created").on(key.createdAt)],
    },
);

/** A consumed device key proof retained for replay protection across service instances. */
export const authenticationReplay = defineTable(
    "authentication_replay",
    {
        /** The digest of the proof's key thumbprint and identifier. */
        id: text("id").primaryKey(),
        /** The earliest safe deletion time in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    {
        tier: "global",
        constraints: (replay) => [index("authentication_replay_expiry").on(replay.expiresAt)],
    },
);

/** A persisted authentication signing key. */
export type SigningKey = Select<typeof signingKey>;
/** A persisted consumed device key proof. */
export type AuthenticationReplay = Select<typeof authenticationReplay>;
