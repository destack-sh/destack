import { identifier, index, integer, type Select, table, text } from "@destack/db";

/** A rotating authentication signing key. */
export const signingKey = table(
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
    (key) => [index("signing_key_created").on(key.createdAt)],
);

/** A consumed DPoP JWT retained for replay protection across service instances. */
export const authenticationReplay = table(
    "authentication_replay",
    {
        /** The verifier's digest of the DPoP public key and JWT identifier. */
        id: text("id").primaryKey(),
        /** The earliest safe deletion time in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    (replay) => [index("authentication_replay_expiry").on(replay.expiresAt)],
);

/** A persisted authentication signing key. */
export type SigningKey = Select<typeof signingKey>;
/** A persisted consumed DPoP JWT. */
export type AuthenticationReplay = Select<typeof authenticationReplay>;
