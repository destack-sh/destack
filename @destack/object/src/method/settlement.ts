import type { ObjectStore } from "../server/store.ts";
import { AccessContext, Caller } from "@destack/access";
import {
    type DatabaseConnection,
    defineTable,
    integer,
    json,
    Snapshot,
    type Table,
    type RelationalQueryBuilder,
    text,
} from "@destack/db";
import { schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import type { ObjectType } from "../object/object.ts";
import type { ObjectCall, ResultsOf } from "./call.ts";

/** The calls whose external work settles once. */
export const settlement = defineTable(
    "settlement",
    {
        /** The settlement's identifier, one per prepared call. */
        id: text("id").primaryKey(),
        /** The idempotency key the call's external work ran under. */
        key: text("key").notNull(),
        /** The object type the call acts on, by name. */
        object: text("object").notNull(),
        /** The method's name. */
        method: text("method").notNull(),
        /** The scope the call acts in. */
        scope: text("scope").notNull(),
        /** The object the call acts on or creates. */
        target: text("target"),
        /** The prepared value as JSON, null until the prepare phase returns. */
        prepared: json("prepared", schema.object({ value: schema.json() })),
        /** When the call's transaction committed, in UTC epoch milliseconds, null until then. */
        committedAt: integer("committed_at"),
        /** When the call reserved the settlement, in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** When a settler last claimed it, null while unclaimed. */
        claimedAt: integer("claimed_at"),
    },
    { log: {} },
);

/** A call whose prepared work settles, built alike in process and in recovery. */
export class SettlementCall {
    /** The object type. */
    readonly object: ObjectType;
    /** The method's name. */
    readonly name: string;
    /** The scope the call acted in. */
    readonly scope: string;
    /** The object the call acted on or created, absent for none. */
    readonly id: string | undefined;
    /** The idempotency key the external work ran under. */
    readonly idempotencyKey: string;
    /** The database the store keeps the call's rows in. */
    readonly database: DatabaseConnection;
    /** When the settlement runs, in UTC epoch milliseconds. */
    readonly now: number;
    /** The store settling the call, which the work's follow-ups run as its principal through. */
    readonly store: ObjectStore;

    /** Keep the recorded call. */
    constructor(fields: {
        readonly object: ObjectType;
        readonly name: string;
        readonly scope: string;
        readonly id: string | undefined;
        readonly idempotencyKey: string;
        readonly database: DatabaseConnection;
        readonly now: number;
        readonly store: ObjectStore;
    }) {
        // keep the recorded fields
        this.object = fields.object;
        this.name = fields.name;
        this.scope = fields.scope;
        this.id = fields.id;
        this.idempotencyKey = fields.idempotencyKey;
        this.database = fields.database;
        this.now = fields.now;
        this.store = fields.store;
    }

    /** Read a table's rows in the store's live rows. */
    read<Read extends Table>(table: Read): RelationalQueryBuilder<Read> {
        return Snapshot.live(this.database).read(table);
    }

    /** The store's principal acting as itself, such as its space or installation. */
    get self(): AccessContext {
        return AccessContext.of(Caller.of(this.store.principal), this.now);
    }

    /** Read through a method as a caller, in its own transaction. */
    query<Result>(caller: AccessContext, call: ObjectCall<Result>): Promise<Result> {
        return this.store.query(caller, call);
    }

    /** Run calls as a caller in one request of their own, committed apart from the settlement. */
    mutate<const Calls extends readonly ObjectCall[]>(
        caller: AccessContext,
        calls: Calls,
    ): Promise<ResultsOf<Calls>> {
        return this.store.mutate(caller, { id: RequestId.create(this.now), calls });
    }

    /** Read the object the call acted on or created, failing for a call without one. */
    requireId(): string {
        if (this.id === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} has no object to settle`);
        }

        return this.id;
    }
}

/** How prepared work settles: committed, or rolled back with its recorded value, if recorded. */
export type Settled<Prepared = unknown> =
    | { readonly status: "committed"; readonly prepared: Prepared }
    | { readonly status: "rolledBack"; readonly prepared: Prepared | undefined };
