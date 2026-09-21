import {
    foreignKey,
    identifier,
    primaryKey,
    recordColumns,
    type Select,
    table,
    text,
    unique,
} from "@destack/db";
import { account } from "./account.ts";
import { accountMembership } from "./membership.ts";

/** An account-local collection of members receiving role grants. */
export const group = table(
    "group",
    {
        ...recordColumns("group"),
        /** The account defining the group. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id),
        /** The account-local group name. */
        name: text("name").notNull(),
    },
    (group) => [
        unique("group_account_name").on(group.accountId, group.name),
        unique("group_account_id").on(group.accountId, group.id),
    ],
);

/** A member's inclusion in an account-local group. */
export const groupMembership = table(
    "group_membership",
    {
        /** The account containing the group and membership. */
        accountId: identifier("account_id", "account").notNull(),
        /** The group receiving the member. */
        groupId: identifier("group_id", "group").notNull(),
        /** The account membership included in the group. */
        accountMembershipId: identifier("account_membership_id", "account-membership").notNull(),
    },
    (entry) => [
        primaryKey({ columns: [entry.groupId, entry.accountMembershipId] }),
        foreignKey({
            columns: [entry.accountId, entry.groupId],
            foreignColumns: [group.accountId, group.id],
        }).onDelete("cascade"),
        foreignKey({
            columns: [entry.accountId, entry.accountMembershipId],
            foreignColumns: [accountMembership.accountId, accountMembership.id],
        }).onDelete("cascade"),
    ],
);

/** An account-local group. */
export type Group = Select<typeof group>;
/** A member's group inclusion. */
export type GroupMembership = Select<typeof groupMembership>;
