import { Filter, type JsonCondition } from "@destack/db";
import { defineSchema, schema } from "@destack/schema";
import { defineProcedure, defineService, eventIterator } from "@destack/service";
import { Event } from "../kind/kind.ts";
import { EventCursor } from "../query/query.ts";
import { SERIES_FOLDS } from "../store/series.ts";
import type {} from "@destack/package/import-meta";

/** The most events one page returns. */
const PAGE_LIMIT = 1000;

/** A filter's text, as the field query language writes it, map entries named by their dotted names. */
export const FilterText = Object.assign(schema.string().min(1).max(1000), {
    /** Read a filter's text as the condition it writes, every event when absent. */
    condition(text: string | undefined): JsonCondition | undefined {
        return text === undefined ? undefined : Filter.parse(text, { isRelational: false });
    },
});

/** The events of a scope a read selects. */
export const EventSelection = defineSchema(
    schema.object({
        /** The kind's name. */
        kind: schema.string().min(1),
        /** The scope whose events to read. */
        scope: schema.string().min(1),
        /** The object a read narrows to, the whole scope when absent. */
        object: schema.string().min(1).exactOptional(),
        /** Whether to read the copies routed from the scopes within the scope beside its own. */
        within: schema.boolean().exactOptional(),
        /** The events selected, as a filter over the kind's keys, every event when absent. */
        where: FilterText.exactOptional(),
        /** Text the events' data contains, ignoring case. */
        text: schema.string().min(1).max(200).exactOptional(),
    }),
);
/** The events of a scope a read selects. */
export type EventSelection = schema.Infer<typeof EventSelection>;

/** A read of a page of events in a time range, newest first unless asked otherwise. */
export const EventQuery = defineSchema(
    EventSelection.extend({
        /** The earliest time, in Unix microseconds. */
        from: schema.number().int().exactOptional(),
        /** The time before which to read, in Unix microseconds. */
        before: schema.number().int().exactOptional(),
        /** The cursor the previous page ended at. */
        cursor: EventCursor.exactOptional(),
        /** The most events to return, 100 when absent. */
        limit: schema.number().int().min(1).max(PAGE_LIMIT).exactOptional(),
        /** The order of the page, newest first when absent. */
        order: schema.enum(["ascending", "descending"]).exactOptional(),
    }),
);
/** A read of a page of events. */
export type EventQuery = schema.Infer<typeof EventQuery>;

/** A page of events and the cursor the next page continues after. */
export const EventResult = defineSchema(
    schema.object({
        /** The events, in the page's order. */
        events: schema.array(Event),
        /** The cursor after the last event, absent once no event follows. */
        cursor: EventCursor.exactOptional(),
    }),
);
/** A page of events. */
export type EventResult = schema.Infer<typeof EventResult>;

/** A fold of the events a selection names in a time range, per step and group of key values. */
export const EventSeries = defineSchema(
    EventSelection.omit({ text: true }).extend({
        /** The key folded, needed by every fold but count. */
        measure: schema.string().min(1).exactOptional(),
        /** The fold. */
        fold: schema.enum(SERIES_FOLDS),
        /** The keys each value of gets a series of its own. */
        group: schema.array(schema.string().min(1)).max(4),
        /** The step width, in milliseconds, at least a minute, one step over the range when absent. */
        step: schema.number().int().min(60_000).exactOptional(),
        /** The earliest time, in Unix microseconds. */
        from: schema.number().int(),
        /** The time before which to fold, in Unix microseconds. */
        before: schema.number().int(),
    }),
);
/** A fold of events per step and group. */
export type EventSeries = schema.Infer<typeof EventSeries>;

/** Folded series, each group's steps oldest first. */
export const SeriesResult = defineSchema(
    schema.object({
        /** The series, one per group of key values. */
        series: schema.array(
            schema.object({
                /** The value of each grouping key. */
                group: schema.record(
                    schema.string(),
                    schema.union([schema.string(), schema.number()]).nullable(),
                ),
                /** The steps holding events. */
                steps: schema.array(
                    schema.object({
                        /** The step's start, in Unix microseconds. */
                        start: schema.number(),
                        /** The folded value. */
                        value: schema.number(),
                        /** The events folded. */
                        events: schema.number().int(),
                    }),
                ),
            }),
        ),
    }),
);
/** Folded series. */
export type SeriesResult = schema.Infer<typeof SeriesResult>;

/** Copies of a kind's events another host routed to the scopes this host keeps. */
export const EventDelivery = defineSchema(
    schema.object({
        /** The kind's name. */
        kind: schema.string().min(1),
        /** The routed events. */
        events: schema.array(Event),
    }),
);
/** Copies of a kind's events another host routed here. */
export type EventDelivery = schema.Infer<typeof EventDelivery>;

/** A procedure that checks the read permission of the kind it reads in its handler. */
const procedure = defineProcedure({ authentication: "identity", permission: null, audit: false });

/** The event service: a scope's readable events, and routed copies received. */
export const eventService = defineService("event", {
    query: procedure
        .route({ method: "POST", path: "/events/query" })
        .input(EventQuery)
        .output(EventResult),
    tail: procedure
        .route({ method: "POST", path: "/events/tail" })
        .input(EventSelection)
        .output(eventIterator(Event)),
    series: procedure
        .route({ method: "POST", path: "/events/series" })
        .input(EventSeries)
        .output(SeriesResult),
    export: procedure
        .route({ method: "POST", path: "/events/export" })
        .input(EventQuery.omit({ cursor: true, limit: true, order: true }))
        .output(eventIterator(Event)),
    receive: procedure
        .route({ method: "POST", path: "/events/receive" })
        .input(EventDelivery)
        .output(schema.object({ /** The events received. */ received: schema.int().min(0) })),
});
