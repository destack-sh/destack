import { type DatabaseConnection, defineTable, integer, json, text } from "@destack/db";
import { schema } from "@destack/schema";
import type { ObjectType } from "../object/object.ts";

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

/** A call whose prepared work commits or rolls back, built from its settlement alike in process and in recovery. */
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
    /** The database, read and written as the system. */
    readonly database: DatabaseConnection;
    /** When the settlement runs, in UTC epoch milliseconds. */
    readonly now: number;

    /** Keep the recorded call. */
    constructor(fields: {
        readonly object: ObjectType;
        readonly name: string;
        readonly scope: string;
        readonly id: string | undefined;
        readonly idempotencyKey: string;
        readonly database: DatabaseConnection;
        readonly now: number;
    }) {
        // keep the recorded fields
        this.object = fields.object;
        this.name = fields.name;
        this.scope = fields.scope;
        this.id = fields.id;
        this.idempotencyKey = fields.idempotencyKey;
        this.database = fields.database;
        this.now = fields.now;
    }

    /** Read the object the call acted on or created, failing for a call without one. */
    requireId(): string {
        if (this.id === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} settles no object`);
        }

        return this.id;
    }
}
