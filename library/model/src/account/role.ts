import { identifier, recordColumns, type Select, table, text, unique } from "@destack/db";
import { account } from "./account.ts";

/** A reusable account-local collection of permissions. */
export const role = table("role", {
    ...recordColumns("role"),
    /** The account defining this role. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id),
    /** The account-local role name. */
    name: text("name").notNull(),
    /** The purpose shown when granting the role. */
    description: text("description").notNull(),
}, (role) => [
    unique("role_account_name").on(role.accountId, role.name),
    unique("role_account_id").on(role.accountId, role.id),
]);

/** An account-defined role. */
export type Role = Select<typeof role>;
