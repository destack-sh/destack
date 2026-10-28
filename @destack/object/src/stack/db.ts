import { JournalEntry } from "../audit/execution.ts";
import {
    bucket,
    defineTable,
    identifier,
    index,
    integer,
    json,
    lease,
    lock,
    text,
    TABLE,
    uniqueIndex,
    BLOB_TABLES,
} from "@destack/db";
import type { DatabaseDefinition, Table } from "@destack/db";
import { accessTables } from "@destack/access";
import { settlement } from "../method/settlement.ts";
import { followKey } from "../replica/key.ts";
import { Follow } from "../replica/shape.ts";
import type { PackageHandle } from "../declare/package.ts";
import { ObjectType } from "../object/object.ts";

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

/** Each object store's system tables, from access to follow keys. */
export const SYSTEM_TABLES: readonly Table[] = [
    ...accessTables,
    ...BLOB_TABLES,
    journal,
    settlement,
    lease,
    lock,
    bucket,
    followKey,
];

/** List a store's tables: system tables, what it serves, follows and needs, and its host's. */
export function tablesOf(store: {
    /** The packages the store serves. */
    readonly packages: readonly Pick<PackageHandle, "objects" | "tables">[];
    /** The shapes the store follows. */
    readonly follows: readonly Follow[];
    /** The tables the store's host stores in its database, such as its keys. */
    readonly tables: readonly Table[];
}): Pick<DatabaseDefinition, "tables" | "replicated"> {
    // replicate the followed shapes' types, whose rows live at their home even where served here
    const replicated = new Set(
        store.follows.flatMap((follow) => Follow.types(follow)).map((type) => type.table),
    );

    // take the rest: the served packages' durable and event types, their tables and the host's
    const types = store.packages
        .flatMap((handle) => Object.values(handle.objects))
        .filter((declared) => declared instanceof ObjectType);
    const local = new Set<Table>([
        ...SYSTEM_TABLES,
        ...types
            .filter((type) => type.storage === "durable" || type.storage === "events")
            .flatMap((type) => type.tables),
        ...store.packages.flatMap((handle) => handle.tables),
        ...store.tables,
    ]);

    // add the tables each table's aggregates and dependents read, such as an issue's comments
    for (const table of local) {
        const { aggregates, dependents } = table[TABLE];
        for (const aggregate of aggregates) {
            local.add("into" in aggregate ? aggregate.into() : aggregate.from());
        }
        for (const dependent of dependents) {
            local.add(dependent.from());
        }
    }

    return {
        tables: [...local].filter((table) => !replicated.has(table)),
        replicated: [...replicated],
    };
}
