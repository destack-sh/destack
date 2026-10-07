import {
    boolean,
    defineTable,
    identifier,
    index,
    integer,
    json,
    text,
    uniqueIndex,
    type Table,
} from "@destack/db";
import { AuditCall } from "../record/call.ts";
import { auditCall, auditTarget, auditEnclosure } from "../record/table.ts";

/** The audit history tables. */
export const auditTables: readonly Table[] = [auditCall, auditTarget, auditEnclosure];

/** The calls a service database executed, kept for retries and delivered to the audit history. */
export const journal = defineTable(
    "journal",
    {
        /** The call's identity. */
        id: identifier("id", "call").primaryKey(),
        /** The scope whose history receives the call. */
        scope: text("scope").notNull(),
        /** The object type and method, such as page.create. */
        method: text("method").notNull(),
        /** The caller of the request the call belongs to, whose clients follow its outcome. */
        caller: text("caller"),
        /** The caller, scope and request a retry replays the call under, absent for outcomes a retry runs again. */
        request: text("request"),
        /** The call's place in its request. */
        position: integer("position").notNull(),
        /** Whether the audit history receives the call. */
        isAudited: boolean("is_audited").notNull(),
        /** The complete call with its execution. */
        call: json("call", AuditCall).notNull(),
        /** The start time, in UTC epoch milliseconds. */
        startedAt: integer("started_at").notNull(),
        /** The end time, in UTC epoch milliseconds, absent while the call runs. */
        finishedAt: integer("finished_at"),
        /** The time the call leaves the journal once delivered, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The time the history accepted the call as it is now, absent while pending. */
        deliveredAt: integer("delivered_at"),
    },
    {
        log: {},
        constraints: (entry) => [
            uniqueIndex("journal_request").on(entry.request, entry.position),
            index("journal_scope_time").on(entry.scope, entry.startedAt),
            index("journal_caller").on(entry.scope, entry.caller),
            index("journal_delivery").on(entry.isAudited, entry.deliveredAt, entry.startedAt),
            index("journal_expiry").on(entry.expiresAt),
        ],
    },
);
