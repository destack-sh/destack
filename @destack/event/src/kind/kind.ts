import { integer, json, TABLE, text, type Row, type Table, type TableColumnMap } from "@destack/db";
import type { Package } from "@destack/package";
import { defineSchema, JsonValue, schema, toJsonSchema } from "@destack/schema";
import { Histogram } from "./histogram.ts";

/** The microseconds in a millisecond, the unit of event times. */
const MICROSECONDS_PER_MILLISECOND = 1000;

/** The query keys of an event kind: a schema per key. */
export type EventKeyShape = Readonly<Record<string, schema.Schema>>;

/** The values of an event's query keys. */
export type EventKeyValues<Shape extends EventKeyShape> = schema.Infer<schema.Object<Shape>>;

/** The entries of a map key, such as an OpenTelemetry record's attributes, by name. */
export type EventKeyMap = Readonly<Record<string, string | number | boolean>>;

/** The value a query key holds: text, a number, a map of entries, or nothing. */
export type EventKeyValue = string | number | EventKeyMap | null;

/** The value of a map key's entry. */
const EntryValue = schema.union([schema.string(), schema.number(), schema.boolean()]);

/** One append-only event of a scope: its identity, time, query keys and data. */
export interface Event<
    Shape extends EventKeyShape = EventKeyShape,
    Data extends JsonValue = JsonValue,
> {
    /** The scope the event happened in, its home. */
    readonly scope: string;
    /** The event's identity, which makes appending it again change nothing. */
    readonly id: string;
    /** When the event happened, in Unix microseconds. */
    readonly time: number;
    /** The values of the kind's query keys. */
    readonly keys: EventKeyValues<Shape>;
    /** The data. */
    readonly data: Data;
}

/** Times of events, in Unix microseconds. */
export const EventTime = {
    /** Convert Unix milliseconds to an event time. */
    of(milliseconds: number): number {
        return milliseconds * MICROSECONDS_PER_MILLISECOND;
    },

    /** Convert an event time to Unix milliseconds, rounding down. */
    milliseconds(time: number): number {
        return Math.floor(time / MICROSECONDS_PER_MILLISECOND);
    },
};

/** An event of any kind as it is kept, its personal values sealed. */
export const Event = defineSchema(
    schema.object({
        /** The scope the event happened in, its home. */
        scope: schema.string().min(1),
        /** The event's identity. */
        id: schema.string().min(1),
        /** When the event happened, in Unix microseconds. */
        time: schema.number().int().nonnegative(),
        /** The values of the kind's query keys, a map key's as its entries. */
        keys: schema.record(
            schema.string(),
            schema
                .union([
                    schema.string(),
                    schema.number(),
                    schema.record(schema.string(), EntryValue),
                ])
                .nullable(),
        ),
        /** The data. */
        data: schema.json(),
    }),
);

/** How an event is kept: exactly once in the caller's transaction, or at most once outside it. */
export const Delivery = defineSchema(schema.enum(["exactly-once", "at-most-once"]));
/** Whether an event is kept exactly once or at most once. */
export type Delivery = schema.Infer<typeof Delivery>;

/** When a scope's hot events flush into a segment: at an age of the oldest, or at a count. */
export const FlushPolicy = defineSchema(
    schema.object({
        /** The longest an event stays hot, in milliseconds. */
        maxAge: schema.number().int().positive(),
        /** The most hot events a scope keeps of the kind. */
        maxRows: schema.number().int().positive(),
    }),
);
/** When a scope's hot events flush into a segment. */
export type FlushPolicy = schema.Infer<typeof FlushPolicy>;

/** When a scope's hot events flush and how long its segments stay. */
export interface EventPolicy {
    /** When hot events flush. */
    readonly flush: FlushPolicy;
    /** How long segments are kept, in milliseconds. */
    readonly retention: number;
}

/** The column type of a query key: text, a whole number, a number, or a JSON map of entries. */
export type EventKeyType = "text" | "integer" | "real" | "json";

/** Build the columns every event kind's table has beside one per key. */
export function eventColumns() {
    return {
        /** The scope the event happened in, its home. */
        scope: text("scope").notNull(),
        /** The event's identity, unique among the kind's events. */
        id: text("id").primaryKey(),
        /** When the event happened, in Unix microseconds. */
        time: integer("time").notNull(),
        /** The data, its personal values sealed. */
        data: json("data", schema.json()).notNull(),
    };
}

/** The columns every event kind's table has. */
type EventColumnMap = TableColumnMap<ReturnType<typeof eventColumns>>;

/** An event kind's table: the event columns, and one per key. */
export type EventTable = Table & EventColumnMap;

/** Tables of event kinds. */
export const EventTable = {
    /** Report whether a table has every event column. */
    is(table: Table): table is EventTable {
        return Object.keys(eventColumns()).every((name) => Object.hasOwn(table, name));
    },
};

/** An event kind as the engine keeps it: the object type's table, keys, data and settings. */
export interface EventKindDescription<Shape extends EventKeyShape, Data extends JsonValue> {
    /** The declaring package. */
    readonly package: Package;
    /** The package-local name, such as call or log. */
    readonly name: string;
    /** The append-only table keeping the hot events, a column per key beside the event columns. */
    readonly table: EventTable;
    /** The exact query keys, each an indexed column. */
    readonly keys: schema.Object<Shape>;
    /** The schema of each event's data. */
    readonly data: schema.Schema<Data>;
    /** Whether every event is kept exactly once or at most once. */
    readonly delivery: Delivery;
    /** When hot events flush and how long segments stay, unless a store sets others per scope. */
    readonly policy: EventPolicy;
    /** Whether segments are locked and chained for the retention, as audit trails are. */
    readonly isLocked?: boolean;
    /** The text key of the person whose key seals personal values, such as an actor. */
    readonly subject?: string;
    /** The data entry recording a distribution of values, which quantile folds merge. */
    readonly distribution?: string;
}

/** A kind of append-only event of scopes. */
export class EventKind<
    Shape extends EventKeyShape = EventKeyShape,
    Data extends JsonValue = JsonValue,
> {
    /** The declaring package. */
    readonly package: Package;
    /** The name, keys, data, delivery, policy, lock and subject. */
    readonly #definition: EventKindDescription<Shape, Data>;
    /** The append-only table keeping the hot events. */
    readonly table: EventTable;
    /** The query key names in declaration order. */
    readonly keys: readonly string[];
    /** The column type of each query key. */
    readonly keyTypes: Readonly<Record<string, EventKeyType>>;
    /** The names of the keys with maps of entries. */
    readonly mapKeys: readonly string[];

    /** Keep a kind, refusing keys, a subject or a distribution the engine cannot keep. */
    constructor(definition: EventKindDescription<Shape, Data>) {
        // check the policy, the subject and the distribution
        EventKind.#require(definition);

        // read the keys and their column types once
        this.package = definition.package;
        this.#definition = definition;
        this.table = definition.table;
        this.keys = Object.keys(definition.keys.shape);
        const types = this.keys.map((name): [string, EventKeyType] => [
            name,
            EventKind.#keyType(definition, name),
        ]);
        this.keyTypes = Object.fromEntries(types);
        this.mapKeys = types.filter(([, type]) => type === "json").map(([name]) => name);
    }

    /** The package-local name. */
    get name(): string {
        return this.#definition.name;
    }

    /** The kind's identity across packages: its package and name. */
    get key(): string {
        return `${this.package.id}/${this.#definition.name}`;
    }

    /** The key of the person whose key seals personal values, absent for a kind without one. */
    get subject(): string | undefined {
        return this.#definition.subject;
    }

    /** Replace each sensitive value by what `map` makes of it. */
    mapSensitive(
        data: JsonValue,
        map: (value: JsonValue, sensitivity: schema.Sensitivity) => JsonValue | undefined,
    ): JsonValue {
        return schema.mapSensitive(this.#definition.data, data, map) ?? null;
    }

    /** The data entry recording a distribution of values, absent for a kind recording none. */
    get distribution(): string | undefined {
        return this.#definition.distribution;
    }

    /** Read the distribution an event's data records, absent where it records none. */
    readDistribution(data: JsonValue): Histogram | undefined {
        // read the kind's distribution entry of the data
        const entry = this.#definition.distribution;
        const recorded = entry === undefined || !JsonValue.isObject(data) ? undefined : data[entry];

        return recorded === undefined ? undefined : Histogram.parse(recorded);
    }

    /** Whether every event is kept exactly once or at most once. */
    get delivery(): Delivery {
        return this.#definition.delivery;
    }

    /** Check an event's query keys against the kind. */
    parseKeys(keys: unknown): EventKeyValues<Shape> {
        return this.#definition.keys.parse(keys);
    }

    /** Check an event's data against the kind. */
    parseData(data: unknown): Data {
        return this.#definition.data.parse(data);
    }

    /** When hot events flush and how long segments stay by default. */
    get policy(): EventPolicy {
        return this.#definition.policy;
    }

    /** Whether segments are locked, chained and never compacted. */
    get isLocked(): boolean {
        return this.#definition.isLocked === true;
    }

    /** Write an event as a row of the kind's table, each query key a column. */
    row(event: Event): Row {
        return {
            ...this.keyValues(event.keys),
            scope: event.scope,
            id: event.id,
            time: event.time,
            data: event.data,
        };
    }

    /** Read a row of the kind's table as the event it keeps, its query keys typed by the kind. */
    event(row: Readonly<Record<string, unknown>>): Event<Shape> {
        // require the columns every event has
        const { scope, id, time, data } = row;
        if (typeof scope !== "string" || typeof id !== "string" || typeof time !== "number") {
            throw new TypeError(`a ${this.key} row has no scope, identity or time`);
        }

        // read each query key's column
        return {
            scope,
            id,
            time,
            keys: this.parseKeys(this.keyValues(row)),
            data: schema.json().parse(data),
        };
    }

    /** Read the query keys' values from a record, null where absent, with map entries parsed. */
    keyValues(values: Readonly<Record<string, unknown>>): Readonly<Record<string, EventKeyValue>> {
        return Object.fromEntries(
            this.keys.map((name) => [name, toKeyValue(this.keyTypes[name], values[name])]),
        );
    }

    /** Read the column type of a query key from its column in the kind's table. */
    static #keyType(
        definition: EventKindDescription<EventKeyShape, JsonValue>,
        name: string,
    ): EventKeyType {
        const kind = definition.table[TABLE].column(name).definition.kind;
        if (kind !== "text" && kind !== "integer" && kind !== "real" && kind !== "json") {
            throw new TypeError(
                `event kind ${definition.name} keeps key ${name} in a ${kind} column: keys must be text, numbers or maps`,
            );
        }

        return kind;
    }

    /** Refuse a policy, a subject or a distribution the engine cannot keep. */
    static #require(definition: EventKindDescription<EventKeyShape, JsonValue>): void {
        // check the policy and each key's column type
        FlushPolicy.parse(definition.policy.flush);
        schema.number().int().positive().parse(definition.policy.retention);
        for (const name of Object.keys(definition.keys.shape)) {
            EventKind.#keyType(definition, name);
        }

        // require the subject to be a text key
        const { subject } = definition;
        if (
            subject !== undefined &&
            (!Object.hasOwn(definition.keys.shape, subject) ||
                EventKind.#keyType(definition, subject) !== "text")
        ) {
            throw new TypeError(`event kind ${definition.name} must have a text subject key`);
        }

        // require the distribution to be an entry of the data
        const { distribution } = definition;
        const entries = toJsonSchema(definition.data).properties ?? {};
        if (distribution !== undefined && !Object.hasOwn(entries, distribution)) {
            throw new TypeError(
                `event kind ${definition.name} has no data entry ${distribution} for its distribution`,
            );
        }
    }
}

/** Read one query key's value as its column keeps it: null if absent, map entries parsed. */
function toKeyValue(type: EventKeyType | undefined, value: unknown): EventKeyValue {
    // an absent key
    if (value === null || value === undefined) {
        return null;
    }
    // a map of entries
    else if (type === "json") {
        return schema.record(schema.string(), EntryValue).parse(value);
    }
    // text or a number
    else if (typeof value === "string" || typeof value === "number") {
        return value;
    }
    // refuse any other value
    else {
        throw new TypeError(`an event key must be text, a number or a map, not ${typeof value}`);
    }
}
