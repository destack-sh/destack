import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { user } from "./user.ts";

/** A user's encrypted TOTP secret and recovery codes. */
export const twoFactor = defineObject({
    name: "two-factor",
    plural: "twoFactors",
    scope: user,
    fields: {
        /** The encrypted TOTP secret. */
        secret: field.string().sensitive(),
        /** The encrypted recovery codes in Better Auth's format, compared by value on each use. */
        backupCodes: field.string().sensitive("deterministic"),
        /** The user required to complete the second factor. */
        user: field.reference(user, { delete: "cascade" }),
        /** Whether enrollment has been confirmed. */
        verified: field.boolean().default(true),
        /** The consecutive failed verification count. */
        failedVerificationCount: field.integer().default(0),
        /** The lockout deadline in epoch milliseconds. */
        lockedUntil: field.time().optional(),
    },
    permissions: ["read"],
    constraints: {
        user: { kind: "index", on: ["userId"] },
    },
}).methods();
/** A persisted second-factor enrollment. */
export type TwoFactor = Select<typeof twoFactor.table>;
