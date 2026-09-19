import {
    identifier,
    recordColumns,
    type Select,
    table,
    text,
    unique,
    uniqueIndex,
} from "@destack/db";
import { account } from "./account.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

/** A reusable collection of account permissions. */
export const role = table("role", {
    ...recordColumns("role"),
    ...provenanceColumns(),
    /** The account defining this role. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id),
    /** The account-local role name. */
    name: text("name").notNull(),
    /** The purpose shown when granting the role. */
    description: text("description").notNull(),
}, (role) => [
    ...provenanceChecks("role", role),
    uniqueIndex("role_scope_name").on(
        role.accountId,
        role.name,
    ),
    unique("role_account_id").on(role.accountId, role.id),
]);

/** An account-defined role. */
export type Role = Select<typeof role>;
