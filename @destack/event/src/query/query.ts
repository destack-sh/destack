import { Condition, Expression, type JsonCondition, type Namespace, Predicate } from "@destack/db";
import { defineSchema, type JsonValue, schema } from "@destack/schema";
import type { Event, EventKind } from "../kind/kind.ts";

/** The order a page reads events in: oldest first, or newest first. */
export type EventOrder = "ascending" | "descending";

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

        /** Report whether an event comes past a cursor in a page's order: later ascending, earlier descending. */
        isPast(event: EventCursor, cursor: EventCursor | undefined, order: EventOrder): boolean {
            const compared = cursor === undefined ? 0 : EventCursor.compare(event, cursor);

            return cursor === undefined || (order === "ascending" ? compared > 0 : compared < 0);
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

/** The events of a scope a query selects. */
export interface EventFilter {
    /** The scope. */
    readonly scope: string;
    /** The condition over the keys, map entries as `<key>.<entry>`, and `source`. */
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
    /** List the fields a filter's condition may name. */
    fields(kind: EventKind, filter: EventFilter): ReadonlySet<string> {
        const maps = new Set(kind.mapKeys);
        const scalars = kind.keys.filter((name) => !maps.has(name));

        return new Set(["source", ...scalars, ...EventFilter.entries(kind, filter)]);
    },

    /** List the map key entries a filter's condition names, as `<key>.<entry>`. */
    entries(kind: EventKind, filter: EventFilter): readonly string[] {
        const maps = kind.mapKeys;

        return named(filter.where ?? {}).filter((name) =>
            maps.some((key) => name.startsWith(`${key}.`)),
        );
    },

    /** Render the map entries a condition and a series name as SQL values. */
    namespace(
        kind: EventKind,
        filter: EventFilter,
        series: { readonly text: readonly string[]; readonly numeric: readonly string[] } = {
            text: [],
            numeric: [],
        },
    ): Namespace {
        // read the entries compared with numbers or measured, and every entry named
        const numeric = new Set([
            ...series.numeric,
            ...(filter.where === undefined
                ? []
                : numericFields(Condition.resolve(filter.where, EventFilter.fields(kind, filter)))),
        ]);
        const maps = kind.mapKeys;
        const entries = [
            ...EventFilter.entries(kind, filter),
            ...[...series.text, ...series.numeric].filter((name) =>
                maps.some((key) => name.startsWith(`${key}.`)),
            ),
        ];

        // render each as a scalar read from its key's JSON
        return {
            extras: Object.fromEntries(
                [...new Set(entries)].map((name) => {
                    const separator = name.indexOf(".");
                    const path = Expression.path(
                        Expression.column(name.slice(0, separator)),
                        name.slice(separator + 1),
                    );

                    return [name, Expression.scalar(path, numeric.has(name) ? "real" : "text")];
                }),
            ),
        };
    },

    /** Read an event's fields as conditions see them. */
    flatten(kind: EventKind, event: Pick<Event, "source" | "keys">): Record<string, JsonValue> {
        const flat: Record<string, JsonValue> = { source: event.source };
        for (const [name, value] of Object.entries(kind.keyValues(event.keys))) {
            if (value !== null && typeof value === "object") {
                for (const [entry, held] of Object.entries(value)) {
                    flat[`${name}.${entry}`] = held;
                }
            } else {
                flat[name] = value;
            }
        }

        return flat;
    },

    /** Refuse a filter whose condition names a field the kind's events lack. */
    check(kind: EventKind, filter: EventFilter): void {
        if (filter.where === undefined) {
            return;
        }
        Condition.resolve(filter.where, EventFilter.fields(kind, filter));
    },

    /** Read the keys a condition pins to one value, which prune segments. */
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
                : Predicate.compile(Condition.resolve(where, EventFilter.fields(kind, filter)));

        // hold the time range, then the condition
        return (event) => {
            if (filter.from !== undefined && event.time < filter.from) {
                return false;
            } else if (filter.before !== undefined && event.time >= filter.before) {
                return false;
            } else if (match === undefined) {
                return true;
            } else {
                return Predicate.matches(match, EventFilter.flatten(kind, event));
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

/** List the field names a condition compares, its combinations and negations walked. */
function named(condition: JsonCondition): string[] {
    return Object.entries(condition).flatMap(([name, entry]) => {
        // walk combinations and negations, else take the compared field
        if (name === "AND" || name === "OR" || name === "NOT") {
            return (Array.isArray(entry) ? entry : [entry]).filter(isCondition).flatMap(named);
        }

        return [name];
    });
}

/** Report whether a condition's entry is itself a condition, as combinations and negations hold. */
function isCondition(entry: unknown): entry is JsonCondition {
    return typeof entry === "object" && entry !== null && !Array.isArray(entry);
}

/** List the fields a resolved condition compares with numbers. */
function numericFields(predicate: Predicate): string[] {
    switch (predicate.kind) {
        case "compare":
            return typeof predicate.value === "number" ? [predicate.name] : [];
        case "oneOf":
            return predicate.values.every((value) => typeof value === "number")
                ? [predicate.name]
                : [];
        case "all":
        case "any":
            return predicate.predicates.flatMap(numericFields);
        case "not":
            return numericFields(predicate.predicate);
        case "missing":
        case "like":
        case "exists":
            return [];
    }
}
