import { JournalEntry } from "../audit/execution.ts";
import {
    bucket,
    defineTable,
    identifier,
    index,
    integer,
    json,
    lease,
    text,
    uniqueIndex,
} from "@destack/db";
import type { Table } from "@destack/db";
import { accessTables } from "@destack/access";
import { settlement } from "../method/settlement.ts";
import { followKey } from "../replica/key.ts";

/** The calls a database executed recently, which retries of their requests replay. */
export const journal = defineTable(
    "journal",
    {
        /** The call's identity. */
        id: identifier("id", "call").primaryKey(),
        /** The scope the call ran in. */
        scope: text("scope").notNull(),
        /** The object type and method, such as page.create. */
        method: text("method").notNull(),
        /** The caller of the request the call belongs to, whose clients follow its outcome. */
        caller: text("caller"),
        /** The caller, scope and request a retry replays the call under. */
        request: text("request"),
        /** The call's place in its request. */
        position: integer("position").notNull(),
        /** The complete call with its execution. */
        call: json("call", JournalEntry).notNull(),
        /** The start time, in UTC epoch milliseconds. */
        startedAt: integer("started_at").notNull(),
        /** The end time, in UTC epoch milliseconds, absent while the call runs. */
        finishedAt: integer("finished_at"),
        /** When the call leaves the journal, past its retry lifetime, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    {
        log: {},
        constraints: (entry) => [
            uniqueIndex("journal_request").on(entry.request, entry.position),
            index("journal_scope_time").on(entry.scope, entry.startedAt),
            index("journal_caller").on(entry.scope, entry.caller),
            index("journal_expiry").on(entry.expiresAt),
        ],
    },
);

/** The tables every durable type's database has: access, journal, settlements, leases, buckets. */
export const storeTables: readonly Table[] = [
    ...accessTables,
    journal,
    settlement,
    lease,
    bucket,
    followKey,
];
