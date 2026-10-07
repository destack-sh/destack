import { Condition, type JsonCondition, Predicate } from "@destack/db";
import { defineSchema, type JsonValue, schema } from "@destack/schema";
import type { Event, EventKind } from "../kind/kind.ts";

/** Where a page of events in time order ends: the time and identity of its last event. */
export const EventCursor = Object.assign(
    defineSchema(
        schema.object({
            /** The last event's time, in Unix microseconds. */
            time: schema.number().int().nonnegative(),
            /** The last event's identity. */
            id: schema.string().min(1),
        }),
    ),
    {
        /** Order two events by time, then identity, as pages read them. */
        compare(left: EventCursor, right: EventCursor): number {
            return left.time - right.time || (left.id < right.id ? -1 : left.id > right.id ? 1 : 0);
        },

        /** Report whether an event lies after a cursor in time order. */
        isAfter(event: EventCursor, cursor: EventCursor | undefined): boolean {
            return cursor === undefined || EventCursor.compare(event, cursor) > 0;
        },
    },
);
/** Where a page of events in time order ends. */
export interface EventCursor {
    /** The last event's time, in Unix microseconds. */
    readonly time: number;
    /** The last event's identity. */
    readonly id: string;
}

/** The events of a scope a query selects: a condition over their keys and source, text in their bodies and a time range. */
export interface EventFilter {
    /** The scope. */
    readonly scope: string;
    /** The condition over the kind's query keys and `source`, as `Filter.parse` reads one, every event when absent. */
    readonly where?: JsonCondition;
    /** Text the opened data contains, ignoring case. */
    readonly text?: string;
    /** The earliest time, in Unix microseconds, inclusive. */
    readonly from?: number;
    /** The time the events come before, in Unix microseconds, exclusive. */
    readonly before?: number;
}

/** The selection of events by a filter: the fields its condition may name, its equalities and its match. */
export const EventFilter = {
    /** List the fields a kind's conditions may name: its query keys and the source. */
    fields(kind: EventKind): ReadonlySet<string> {
        return new Set(["source", ...kind.keys]);
    },

    /** Refuse a filter whose condition names a field the kind's events lack. */
    check(kind: EventKind, filter: EventFilter): void {
        if (filter.where === undefined) {
            return;
        }
        Condition.resolve(filter.where, EventFilter.fields(kind));
    },

    /** Read the query keys a filter's condition requires to equal one value, which segments' key sets and bloom filters prune by. */
    equalities(kind: EventKind, filter: EventFilter): Readonly<Record<string, string | number>> {
        return Object.fromEntries(
            Object.entries(filter.where ?? {}).filter(
                (entry): entry is [string, string | number] =>
                    kind.keys.includes(entry[0]) &&
                    (typeof entry[1] === "string" || typeof entry[1] === "number"),
            ),
        );
    },

    /** Build the match of a filter's time range and condition over a kind's stored events, text aside. */
    match(
        kind: EventKind,
        filter: EventFilter,
    ): (event: Pick<Event, "source" | "time" | "keys">) => boolean {
        // compile the condition over the source and the query keys
        const where = filter.where;
        const match =
            where === undefined
                ? undefined
                : Predicate.compile(Condition.resolve(where, EventFilter.fields(kind)));

        // hold the time range, then the condition
        return (event) => {
            if (filter.from !== undefined && event.time < filter.from) {
                return false;
            } else if (filter.before !== undefined && event.time >= filter.before) {
                return false;
            } else if (match === undefined) {
                return true;
            } else {
                return Predicate.matches(match, { ...event.keys, source: event.source });
            }
        };
    },

    /** Report whether an opened data contains a filter's text, ignoring case. */
    contains(filter: EventFilter, data: JsonValue): boolean {
        return (
            filter.text === undefined ||
            JSON.stringify(data).toLowerCase().includes(filter.text.toLowerCase())
        );
    },
};
