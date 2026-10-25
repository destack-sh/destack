import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { user } from "./user.ts";

/** Whether a factor's enrollment is confirmed by a first code. */
export const FACTOR_STATUSES = ["pending", "verified"] as const;

/** A user's enrolled authenticator: its sealed secret, last step, recovery codes and lockout. */
export const factor = defineObject({
    name: "factor",
    plural: "factors",
    scope: user,
    fields: {
        /** The user the factor belongs to. */
        user: field.reference(user, { delete: "cascade" }),
        /** The TOTP secret, sealed by the keyring under the factor label and the user. */
        secret: field.string().sensitive(),
        /** The digests of the unused recovery codes. */
        recoveryCodes: field.json(schema.array(schema.string())).sensitive(),
        /** The step of the last accepted code, which no later code may reuse. */
        lastStep: field.integer().default(0),
        /** The codes tried since the last accepted one, each counted before its check. */
        failures: field.integer().default(0),
        /** When the factor accepts codes again after too many failures, past while it does. */
        lockedUntil: field.time().default(0),
        /** Whether a first code confirmed the enrollment. */
        status: field.enum(FACTOR_STATUSES),
    },
    permissions: ["read"],
    constraints: {
        user: { kind: "unique", on: ["userId"] },
    },
});
/** A persisted authenticator enrollment. */
export type Factor = Select<typeof factor.table>;
