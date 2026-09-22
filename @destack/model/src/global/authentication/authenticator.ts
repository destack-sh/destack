import { boolean, identifier, index, integer, type Select, table, text } from "@destack/db";
import { user } from "../account/user.ts";

/** A WebAuthn credential registered by a user. */
export const passkey = table(
    "passkey",
    {
        /** The credential record identifier. */
        id: identifier("id", "passkey").primaryKey(),
        /** The user-selected credential name. */
        name: text("name"),
        /** The encoded WebAuthn public key. */
        publicKey: text("public_key").notNull(),
        /** The user authenticated by this credential. */
        userId: identifier("user_id", "user")
            .notNull()
            .references(() => user.id, { onDelete: "cascade" }),
        /** The authenticator-assigned credential identifier. */
        credentialID: text("credential_id").notNull().unique(),
        /** The latest authenticator signature counter. */
        counter: integer("counter").notNull(),
        /** The WebAuthn single-device or multi-device credential type. */
        deviceType: text("device_type").notNull(),
        /** Whether the credential is backed up by its authenticator. */
        backedUp: boolean("backed_up").notNull(),
        /** The encoded authenticator transports. */
        transports: text("transports"),
        /** The credential registration time in epoch milliseconds. */
        createdAt: integer("created_at"),
        /** The authenticator model identifier. */
        aaguid: text("aaguid"),
    },
    (credential) => [index("passkey_user").on(credential.userId)],
);

/** A user's encrypted TOTP secret and protected recovery codes. */
export const twoFactor = table(
    "two_factor",
    {
        /** The second-factor record identifier. */
        id: identifier("id", "two-factor").primaryKey(),
        /** The encrypted TOTP secret. */
        secret: text("secret").notNull(),
        /** The protected recovery codes in Better Auth's format. */
        backupCodes: text("backup_codes").notNull(),
        /** The user required to complete the second factor. */
        userId: identifier("user_id", "user")
            .notNull()
            .references(() => user.id, { onDelete: "cascade" }),
        /** Whether enrollment has been confirmed. */
        verified: boolean("verified").default(true),
        /** The consecutive failed verification count. */
        failedVerificationCount: integer("failed_verification_count").default(0),
        /** The lockout deadline in epoch milliseconds. */
        lockedUntil: integer("locked_until"),
    },
    (factor) => [
        index("two_factor_user").on(factor.userId),
        index("two_factor_secret").on(factor.secret),
    ],
);

/** A persisted WebAuthn credential. */
export type Passkey = Select<typeof passkey>;
/** A persisted second-factor enrollment. */
export type TwoFactor = Select<typeof twoFactor>;
