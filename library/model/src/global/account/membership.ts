import { identifier, index, recordColumns, type Select, table, unique } from "@destack/db";
import { account } from "./account.ts";
import { user } from "./user.ts";

/** AccountMembership records. */
export const accountMembership = table("account_membership", {
    ...recordColumns("account-membership"),
    /** The account granting membership. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id, {
        onDelete: "cascade",
    }),
    /** The account member. */
    userId: identifier("user_id", "user").notNull().references(() => user.id, {
        onDelete: "restrict",
    }),
}, (accountMembership) => [
    unique("account_membership_account_id").on(accountMembership.accountId, accountMembership.id),
    unique("account_membership_account_user").on(
        accountMembership.accountId,
        accountMembership.userId,
    ),
    index("account_membership_user").on(accountMembership.userId),
]);

/** A persisted membership record. */
export type AccountMembership = Select<typeof accountMembership>;
