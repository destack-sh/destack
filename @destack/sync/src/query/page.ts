import { Scalar } from "@destack/db/query";
import { defineSchema, schema } from "@destack/schema";
import { LogPosition } from "@destack/db/log";

/** How the source settled one of a subscriber's mutations: executed, or rejected with its failure. */
export const MutationOutcome = defineSchema(
    schema.object({
        /** The mutation's request identifier. */
        id: schema.string().min(1),
        /** The failure the source recorded, absent once executed. */
        error: schema.json().optional(),
    }),
);
/** How the source settled one of a subscriber's mutations: executed, or rejected with its failure. */
export type MutationOutcome = schema.Infer<typeof MutationOutcome>;

/** One row entering, changing within or leaving the rows a subscriber holds, as it travels. */
export const RowChange = defineSchema(
    schema.object({
        /** The row's table, by SQL name. */
        table: schema.string().min(1),
        /** Whether the row enters, changes within or leaves. */
        operation: schema.enum(["insert", "update", "delete"]),
        /** The row's columns by property, as JSON: its readable columns after an insertion or update, its key for a deletion. */
        row: schema.record(schema.string(), schema.json()),
        /** The row before an update, as JSON, which a copy restores to revert a prediction. */
        before: schema.record(schema.string(), schema.json()).optional(),
        /** The columns the subscriber may not read, which the row leaves out and a copy holds as missing. */
        concealed: schema.array(schema.string()).optional(),
    }),
);
/** One row entering, changing within or leaving the rows a subscriber holds, as it travels. */
export type RowChange = schema.Infer<typeof RowChange>;

/** One group of an aggregate query taking new values or leaving, or every group of the query leaving. */
export const ResultChange = defineSchema(
    schema.object({
        /** The query, by its path of names. */
        query: schema.string().min(1),
        /** The group's values by column property, the held row's join value first for an include; null for every group. */
        group: schema.record(schema.string(), Scalar).nullable(),
        /** The group's measures by name, or null once the group holds no rows. */
        values: schema.record(schema.string(), Scalar).nullable(),
        /** The rows the group holds while it holds any, which let a copy predict when it empties. */
        rows: schema.number().int().positive().optional(),
        /** Each average's sum and count of present values, by measure name, which let a copy add predictions to it. */
        parts: schema
            .record(schema.string(), schema.object({ sum: Scalar, count: schema.number().int() }))
            .optional(),
    }),
);
/** One group of an aggregate query taking new values or leaving, or every group of the query leaving. */
export type ResultChange = schema.Infer<typeof ResultChange>;

/** Rows and aggregates of queries as subscribers receive them: snapshot pages, or whole transactions after a position. */
export const QueryPage = defineSchema(
    schema.object({
        /** Whether the page starts a snapshot, replacing everything the subscriber holds. */
        reset: schema.boolean(),
        /** Whether the subscriber holds all of its queries once it applies the page, false within a snapshot. */
        complete: schema.boolean(),
        /** The rows entering, changing within or leaving the queries. */
        changes: schema.array(RowChange),
        /** The aggregate groups taking new values or leaving. */
        results: schema.array(ResultChange).optional(),
        /** The log position to continue after once the subscriber holds all of its queries. */
        position: LogPosition,
        /** The outcomes of the subscriber's own mutations whose changes the page holds. */
        outcomes: schema.array(MutationOutcome).optional(),
    }),
);
/** Rows and aggregates of queries as subscribers receive them: snapshot pages, or whole transactions after a position. */
export type QueryPage = schema.Infer<typeof QueryPage>;
