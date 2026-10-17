import {
    Condition,
    Expression,
    Filter,
    type JsonCondition,
    type Namespace,
    Predicate,
    SearchText,
    TABLE,
} from "@destack/db";
import { type JsonValue, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { Event, EventKind } from "../kind/kind.ts";

/** The order a page reads events in: oldest first, or newest first. */
export type EventOrder = "ascending" | "descending";

/** Where a page of events in time order ends: the time and identity of its last event. */
export const EventCursor = Object.assign(
    schema.object({
        /** The last event's time, in Unix microseconds. */
        time: schema.number().int().nonnegative(),
        /** The last event's identity. */
        id: schema.string().min(1),
    }),
    {
        /** Order two events by time, then identity, as pages read them. */
        compare(left: EventCursor, right: EventCursor): number {
            return left.time - right.time || (left.id < right.id ? -1 : left.id > right.id ? 1 : 0);
        },

        /** Report whether an event lies after a cursor in time order. */
        isAfter(event: EventCursor, cursor: EventCursor | undefined): boolean {
            return cursor === undefined || EventCursor.compare(event, cursor) > 0;
        },

        /** Write a cursor as the opaque text a list page continues after. */
        write(cursor: EventCursor): string {
            return `${String(cursor.time)}.${cursor.id}`;
        },

        /** Read a cursor from the text a list page answered, refusing text no page writes. */
        read(text: string): EventCursor {
            // split the time from the identity at the first dot
            const separator = text.indexOf(".");
            const time = Number(text.slice(0, separator));
            const parsed = EventCursor.safeParse({ time, id: text.slice(separator + 1) });

            // refuse text without both
            if (separator < 1 || !parsed.success) {
                throw new CallError("BAD_REQUEST", { message: `no event cursor ${text}` });
            }

            return parsed.data;
        },

        /** Report whether an event is past a cursor: later ascending, earlier descending. */
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

/** A filter's text in the field query language, with map entries by their dotted names. */
export const FilterText = Object.assign(schema.string().min(1).max(1000), {
    /** Read a filter's text as the condition it writes, every event when absent. */
    condition(text: string | undefined): JsonCondition | undefined {
        return text === undefined ? undefined : Filter.parse(text, { isRelational: false });
    },
});

/** The events of some scopes a query selects. */
export interface EventFilter {
    /** The scopes, at least one: a scope's own, and its kept replicas of the scopes within it. */
    readonly scopes: readonly [string, ...string[]];
    /** The condition over `id`, `time` and the keys, map entries as `<key>.<entry>`. */
    readonly where?: JsonCondition;
}

/** The time range a filter's condition requires, in Unix microseconds, open where absent. */
export interface EventRange {
    /** The earliest time, inclusive. */
    readonly from?: number;
    /** The time the events come before, exclusive. */
    readonly before?: number;
}

/** The selection of events by a filter: its condition's fields, its equalities and its match. */
export const EventFilter = {
    /** List the fields a filter's condition may compare. */
    fields(kind: EventKind, filter: EventFilter): ReadonlySet<string> {
        const maps = new Set(kind.mapKeys);
        const scalars = kind.keys.filter((name) => !maps.has(name));

        return new Set(["scope", "id", "time", ...scalars, ...EventFilter.entries(kind, filter)]);
    },

    /** List the map key entries a filter's condition references, as `<key>.<entry>`. */
    entries(kind: EventKind, filter: EventFilter): readonly string[] {
        const maps = kind.mapKeys;

        return named(filter.where ?? {}).filter((name) =>
            maps.some((key) => name.startsWith(`${key}.`)),
        );
    },

    /** Render the map entries a condition and a series reference as SQL values. */
    namespace(
        kind: EventKind,
        filter: EventFilter,
        series: { readonly text: readonly string[]; readonly numeric: readonly string[] } = {
            text: [],
            numeric: [],
        },
    ): Namespace {
        // read the entries compared with numbers or measured, and every referenced entry
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
    flatten(
        kind: EventKind,
        event: Pick<Event, "scope" | "id" | "time" | "keys">,
    ): Record<string, JsonValue> {
        const flat: Record<string, JsonValue> = {
            scope: event.scope,
            id: event.id,
            time: event.time,
        };
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

    /** Refuse a filter with a condition on a field the kind's events lack. */
    check(kind: EventKind, filter: EventFilter): void {
        if (filter.where === undefined) {
            return;
        }
        Condition.resolve(filter.where, EventFilter.fields(kind, filter));
    },

    /** Read the keys a condition pins to one value, which prune segments. */
    equalities(kind: EventKind, filter: EventFilter): Readonly<Record<string, string | number>> {
        return Object.fromEntries(
            Condition.conjuncts(filter.where).flatMap((condition) =>
                Object.entries(condition).filter(
                    (entry): entry is [string, string | number] =>
                        kind.keys.includes(entry[0]) &&
                        (typeof entry[1] === "string" || typeof entry[1] === "number"),
                ),
            ),
        );
    },

    /** Build the match of a filter's condition over a kind's events. */
    match(
        kind: EventKind,
        filter: EventFilter,
    ): (event: Pick<Event, "scope" | "id" | "time" | "keys">) => boolean {
        // compile the condition over the event's fields and keys
        const where = filter.where;
        if (where === undefined) {
            return () => true;
        }
        const match = Predicate.compile(Condition.resolve(where, EventFilter.fields(kind, filter)));

        // decide the fields and keys, and the searched keys' words
        return (event) => {
            const flat = EventFilter.flatten(kind, event);

            return (
                match({
                    field: (name) => flat[name],
                    placeholder: Predicate.unbound,
                    exists: () => undefined,
                    search: (words) => kind.table[TABLE].requireFullText().matches(flat, words),
                }) === true
            );
        };
    },

    /** Read the words a filter's condition searches at its top, which prune segments. */
    search(filter: EventFilter): string[] {
        return Condition.conjuncts(filter.where).flatMap((condition) => {
            const searched = condition["SEARCH"];

            return typeof searched === "string" ? SearchText.query(searched) : [];
        });
    },

    /** Read the identifier a filter's condition pins at its top, which prunes segments. */
    identity(filter: EventFilter): string | undefined {
        const pinned = Condition.conjuncts(filter.where).map((condition) => condition["id"]);

        return pinned.find((id): id is string => typeof id === "string");
    },

    /** Read the time range a filter's condition requires at its top, which prunes segments. */
    range(filter: EventFilter): EventRange {
        // read each bound on time the condition and its top conjunction require
        const bounds = Condition.conjuncts(filter.where).flatMap((condition) =>
            readBounds(condition["time"]),
        );
        const froms = bounds.flatMap((bound) => (bound.from === undefined ? [] : [bound.from]));
        const befores = bounds.flatMap((bound) =>
            bound.before === undefined ? [] : [bound.before],
        );

        return {
            ...(froms.length === 0 ? {} : { from: Math.max(...froms) }),
            ...(befores.length === 0 ? {} : { before: Math.min(...befores) }),
        };
    },
};

/** Read the time range one entry on time requires: an equality, or comparisons. */
function readBounds(entry: unknown): readonly EventRange[] {
    // bound an exact time to its microsecond, and skip an entry that bounds nothing
    if (typeof entry === "number") {
        return [{ from: entry, before: entry + 1 }];
    } else if (!Condition.is(entry)) {
        return [];
    }

    // bound each comparison, times being whole microseconds
    const { gte, gt, lt, lte } = entry;

    return [
        ...(typeof gte === "number" ? [{ from: gte }] : []),
        ...(typeof gt === "number" ? [{ from: gt + 1 }] : []),
        ...(typeof lt === "number" ? [{ before: lt }] : []),
        ...(typeof lte === "number" ? [{ before: lte + 1 }] : []),
    ];
}

/** List the field names a condition compares, its combinations and negations walked. */
function named(condition: JsonCondition): string[] {
    return Object.entries(condition).flatMap(([name, entry]) => {
        // walk combinations and negations, else take the compared field
        if (name === "AND" || name === "OR" || name === "NOT") {
            return (Array.isArray(entry) ? entry : [entry]).filter(Condition.is).flatMap(named);
        }

        return [name];
    });
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
        case "characters":
        case "length":
        case "search":
        case "exists":
            return [];
    }
}
