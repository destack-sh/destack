import {
    check,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { account } from "./account.ts";
import { user } from "./user.ts";

/** A single-use invitation to join an account. */
export const accountInvitation = table("account_invitation", {
    ...recordColumns("account-invitation"),
    /** The account accepting a new member. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id),
    /** The inviting user. */
    invitedBy: identifier("invited_by", "user").notNull().references(() => user.id),
    /** The email address the accepting user must verify. */
    email: text("email").notNull(),
    /** The hash of the single-use invitation secret. */
    tokenHash: text("token_hash").notNull().unique(),
    /** The acceptance deadline. */
    expiresAt: integer("expires_at").notNull(),
    /** The authenticated user accepting the invitation. */
    acceptedBy: identifier("accepted_by", "user").references(() => user.id),
    /** The acceptance time. */
    acceptedAt: integer("accepted_at"),
    /** Explicit withdrawal time. */
    revokedAt: integer("revoked_at"),
}, (accountInvitation) => [
    check(
        "account_invitation_expiry",
        sql`${accountInvitation.expiresAt} > ${accountInvitation.createdAt}`,
    ),
    check(
        "account_invitation_acceptance",
        sql`(${accountInvitation.acceptedAt} IS NULL) = (${accountInvitation.acceptedBy} IS NULL) AND (${accountInvitation.acceptedAt} IS NULL OR (${accountInvitation.acceptedAt} >= ${accountInvitation.createdAt} AND ${accountInvitation.acceptedAt} < ${accountInvitation.expiresAt} AND ${accountInvitation.revokedAt} IS NULL))`,
    ),
]);

/** An account invitation. */
export type AccountInvitation = Select<typeof accountInvitation>;
