import { defineTable, integer, json, text, textEnum } from "@destack/db";
import { schema } from "@destack/schema";
import { Failure } from "@destack/error";
import { Call } from "../call/index.ts";

/** The states a mutation passes through in the outbox, in order. */
export const MUTATION_STATES = ["queued", "sent", "executed", "rejected"] as const;

/** The mutations of a client's outbox, in order. */
export const mutation = defineTable("mutation", {
    /** The request identifier. */
    id: text("id").primaryKey().validate(schema.uuidv7()),
    /** The party that queued the mutation, such as a browser tab. */
    origin: text("origin").notNull(),
    /** The mutation's place in the outbox. */
    position: integer("position").notNull().unique(),
    /** The calls. */
    calls: json("calls", schema.array(Call)).notNull(),
    /** The identifier of the branch the calls are pushed to, absent on the main line. */
    branch: text("branch"),
    /** The mutation's state; its calls change only while queued. */
    state: textEnum("state", MUTATION_STATES).notNull().default("queued"),
    /** The server log of the executed mutation. */
    log: text("log"),
    /** The server log's epoch of the executed mutation. */
    epoch: text("epoch"),
    /** The server log sequence of the mutation's changes. */
    sequence: integer("sequence"),
    /** The failure the server recorded. */
    error: json("error", Failure),
});

/** The predictions applied over a client's replica, one row per layer in the order applied. */
export const layer = defineTable("layer", {
    /** The layer: a queued mutation's identifier, or the checked-out branch's. */
    key: text("key").primaryKey(),
    /** The layer's place in the order applied. */
    position: integer("position").notNull().unique(),
    /** The predicted changes, as JSON text. */
    changes: text("changes").notNull(),
    /** The tables the prediction reads or writes, as a JSON array of SQL names. */
    tables: text("tables").notNull(),
});

/** The branch a client's replica shows over the main line. */
export const checkout = defineTable("checkout", {
    /** The one row's key. */
    id: integer("id").primaryKey(),
    /** The identifier of the checked-out branch. */
    branch: text("branch").notNull(),
});

/** The tables of a client's queue and predictions. */
export const predictionTables = [mutation, layer, checkout] as const;
