import { Scalar, LogPosition } from "@destack/db";
import { defineSchema, schema } from "@destack/schema";

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
        before: schema.record(schema.string(), schema.json()).exactOptional(),
        /** The columns the subscriber may not read. */
        concealed: schema.array(schema.string()).exactOptional(),
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
        rows: schema.number().int().positive().exactOptional(),
        /** Each average's sum and count of present values. */
        parts: schema
            .record(schema.string(), schema.object({ sum: Scalar, count: schema.number().int() }))
            .exactOptional(),
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
export const Page = defineSchema(
    schema.object({
        /** Whether the page starts a snapshot. */
        reset: schema.boolean(),
        /** Whether the subscriber has all its queries after the page. */
        complete: schema.boolean(),
        /** The row changes. */
        changes: schema.array(RowChange),
        /** The group changes. */
        results: schema.array(ResultChange).exactOptional(),
        /** The log position to continue after. */
        position: LogPosition,
        /** The events since the last page. */
        broadcasts: schema.array(Broadcast).exactOptional(),
        /** The scopes the subscription reads, nearest first, when its source sends them. */
        scopes: schema.array(schema.string().min(1)).exactOptional(),
    }),
);
/** A page of query rows and aggregates. */
export type Page = schema.Infer<typeof Page>;
