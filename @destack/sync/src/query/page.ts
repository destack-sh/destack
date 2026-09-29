import { Scalar } from "@destack/db/query";
import { defineSchema, schema } from "@destack/schema";
import { LogPosition } from "@destack/db/log";

/** The source's outcome of a subscriber's mutation. */
export const MutationOutcome = defineSchema(
    schema.object({
        /** The mutation's request identifier. */
        id: schema.string().min(1),
        /** The recorded failure, absent once executed. */
        error: schema.json().optional(),
    }),
);
/** The source's outcome of a subscriber's mutation. */
export type MutationOutcome = schema.Infer<typeof MutationOutcome>;

/** One row entering, changing within or leaving a subscriber's rows. */
export const RowChange = defineSchema(
    schema.object({
        /** The row's table, by SQL name. */
        table: schema.string().min(1),
        /** Whether the row enters, changes or leaves. */
        operation: schema.enum(["insert", "update", "delete"]),
        /** The row's columns as JSON, or its key for a deletion. */
        row: schema.record(schema.string(), schema.json()),
        /** The row before an update, as JSON. */
        before: schema.record(schema.string(), schema.json()).optional(),
        /** The columns the subscriber may not read. */
        concealed: schema.array(schema.string()).optional(),
    }),
);
/** One row entering, changing within or leaving a subscriber's rows. */
export type RowChange = schema.Infer<typeof RowChange>;

/** One aggregate group taking new values or leaving, or every group leaving. */
export const ResultChange = defineSchema(
    schema.object({
        /** The query, by its path of names. */
        query: schema.string().min(1),
        /** The group's values, the join value first for an include, null for every group. */
        group: schema.record(schema.string(), Scalar).nullable(),
        /** The group's measures, or null once empty. */
        values: schema.record(schema.string(), Scalar).nullable(),
        /** The group's row count. */
        rows: schema.number().int().positive().optional(),
        /** Each average's sum and count of present values. */
        parts: schema
            .record(schema.string(), schema.object({ sum: Scalar, count: schema.number().int() }))
            .optional(),
    }),
);
/** One aggregate group taking new values or leaving, or every group leaving. */
export type ResultChange = schema.Infer<typeof ResultChange>;

/** An event sent to one topic, never stored. */
export const Broadcast = defineSchema(
    schema.object({
        /** The topic the event was sent to. */
        topic: schema.string().min(1),
        /** The event. */
        event: schema.json(),
    }),
);
/** An event sent to one topic, never stored. */
export type Broadcast = schema.Infer<typeof Broadcast>;

/** A page of query rows and aggregates. */
export const QueryPage = defineSchema(
    schema.object({
        /** Whether the page starts a snapshot. */
        reset: schema.boolean(),
        /** Whether the subscriber holds all its queries after the page. */
        complete: schema.boolean(),
        /** The row changes. */
        changes: schema.array(RowChange),
        /** The group changes. */
        results: schema.array(ResultChange).optional(),
        /** The log position to continue after. */
        position: LogPosition,
        /** The outcomes of the subscriber's mutations. */
        outcomes: schema.array(MutationOutcome).optional(),
        /** The events since the last page. */
        broadcasts: schema.array(Broadcast).optional(),
        /** The scopes the subscription reads, nearest first, when its source sends them. */
        scopes: schema.array(schema.string().min(1)).optional(),
    }),
);
/** A page of query rows and aggregates. */
export type QueryPage = schema.Infer<typeof QueryPage>;
