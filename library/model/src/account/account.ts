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
import { user } from "./user.ts";

/** Account records. */
export const account = table("account", {
    ...recordColumns("account"),
    /** The public account handle. */
    handle: text("handle").notNull().unique(),
    /** The displayed account name. */
    name: text("name").notNull(),
    /** The default jurisdiction copied into newly created spaces. */
    defaultResidency: text("default_residency", { enum: ["eu", "us"] }).notNull(),
    /** Suspension time; retained records remain available for recovery. */
    suspendedAt: integer("suspended_at"),
    /** Explicit deletion request, completed after retention and cleanup. */
    deletionRequestedAt: integer("deletion_requested_at"),
    /** Whether the account belongs to a person or organisation. */
    kind: text("kind", { enum: ["personal", "organisation"] }).notNull(),
    /** The user owning a personal account. */
    userId: identifier("user_id", "user").unique().references(() => user.id, {
        onDelete: "restrict",
    }),
}, (account) => [
    check("account_residency", sql`${account.defaultResidency} IN ('eu', 'us')`),
    check("account_kind", sql`${account.kind} IN ('personal', 'organisation')`),
    check(
        "account_user",
        sql`(${account.kind} = 'personal' AND ${account.userId} IS NOT NULL) OR (${account.kind} = 'organisation' AND ${account.userId} IS NULL)`,
    ),
    check(
        "account_handle",
        sql`length(${account.handle}) BETWEEN 1 AND 63 AND ${account.handle} NOT GLOB '*[^a-z0-9-]*' AND ${account.handle} NOT LIKE '-%' AND ${account.handle} NOT LIKE '%-'`,
    ),
]);

/** A persisted account record. */
export type Account = Select<typeof account>;
