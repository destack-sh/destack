import {
    defineTable,
    index,
    integer,
    json,
    real,
    TABLE,
    text,
    type ColumnBuilder,
    type InsertValue,
} from "@destack/db";
import { ModuleMetadata, Package } from "@destack/package";
import { defineSchema, type JsonValue, schema, toJsonSchema } from "@destack/schema";

/** The query keys of an event kind: a schema per key. */
export type EventKeyShape = Readonly<Record<string, schema.Schema>>;

/** The values of an event's query keys. */
export type EventKeyValues<Shape extends EventKeyShape> = schema.Infer<schema.Object<Shape>>;

/** One append-only event of a scope: its identity, the scope it came from, its time, query keys and data. */
export interface Event<
    Shape extends EventKeyShape = EventKeyShape,
    Data extends JsonValue = JsonValue,
> {
    /** The scope keeping the event. */
    readonly scope: string;
    /** The event's identity within its source, which makes appending it again change nothing. */
    readonly id: string;
    /** The scope the event happened in, its own scope unless another routed it here. */
    readonly source: string;
    /** When the event happened, in Unix microseconds. */
    readonly time: number;
    /** The values of the kind's query keys. */
    readonly keys: EventKeyValues<Shape>;
    /** The data. */
    readonly data: Data;
}

/** An event of any kind as it is kept and routed: its keys as their columns hold them, its data's personal values sealed. */
export const Event = defineSchema(
    schema.object({
        /** The scope keeping the event. */
        scope: schema.string().min(1),
        /** The event's identity within its source. */
        id: schema.string().min(1),
        /** The scope the event happened in. */
        source: schema.string().min(1),
        /** When the event happened, in Unix microseconds. */
        time: schema.number().int().nonnegative(),
        /** The values of the kind's query keys. */
        keys: schema.record(
            schema.string(),
            schema.union([schema.string(), schema.number()]).nullable(),
        ),
        /** The data. */
        data: schema.json(),
    }),
);

/** The scopes a kind's events are copied to beside their own: every scope enclosing theirs, or the account paying for theirs. */
export const Route = defineSchema(schema.enum(["enclosing", "payer"]));
/** The scopes a kind's events are copied to beside their own. */
export type Route = schema.Infer<typeof Route>;

/** Whether an event is kept exactly once, appended in the caller's transaction and never dropped, or at most once, written on its own and dropped when the write fails. */
export const Delivery = defineSchema(schema.enum(["exactly-once", "at-most-once"]));
/** Whether an event is kept exactly once or at most once. */
export type Delivery = schema.Infer<typeof Delivery>;

/** When a scope's hot events flush into a segment: once the oldest reaches an age, or once they reach a count. */
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

/** The column type a query key stores its values in. */
export type EventKeyType = "text" | "integer" | "real";

/** The pattern of an event kind's name, which names its table. */
const KIND_NAME = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/u;

/** The columns every event has, which no query key may name. */
const RESERVED_KEYS: ReadonlySet<string> = new Set(["scope", "id", "source", "time", "data"]);

/** An event kind as its package declares it. */
export interface EventKindDefinition<Shape extends EventKeyShape, Data extends JsonValue> {
    /** The package-local name, such as call or log. */
    readonly name: string;
    /** What the events record. */
    readonly description: string;
    /** The exact query keys, each an indexed column. */
    readonly keys: schema.Object<Shape>;
    /** The schema of each event's data. */
    readonly data: schema.Schema<Data>;
    /** Whether every event is kept exactly once or at most once. */
    readonly delivery: Delivery;
    /** When hot events flush and how long segments stay, unless a store's policy names others for a scope. */
    readonly policy: EventPolicy;
    /** The scopes keeping a copy of each event beside its own: those enclosing it, or the account paying for it. */
    readonly route?: Route;
    /** Whether segments are locked for the retention, chained by digest and never compacted, as audit trails are. */
    readonly isLocked?: boolean;
    /** The text key naming the person whose key seals the data's personal values, such as an actor. */
    readonly subject?: string;
}

/** A kind of append-only event of scopes: its keys, its data, its guarantees and the table keeping its hot events. */
export class EventKind<
    Shape extends EventKeyShape = EventKeyShape,
    Data extends JsonValue = JsonValue,
> {
    /** The declaring package. */
    readonly package: Package;
    /** The name, keys, data, delivery, policy, route, lock and subject. */
    readonly #definition: EventKindDefinition<Shape, Data>;
    /** The append-only table keeping the hot events. */
    readonly table: ReturnType<typeof tableOf>;

    /** Hold a kind and its table. */
    constructor(
        owner: Package,
        definition: EventKindDefinition<Shape, Data>,
        table: ReturnType<typeof tableOf>,
    ) {
        this.package = owner;
        this.#definition = definition;
        this.table = table;
    }

    /** The package-local name. */
    get name(): string {
        return this.#definition.name;
    }

    /** The kind's identity across packages: its package and name. */
    get key(): string {
        return `${this.package.id}/${this.#definition.name}`;
    }

    /** The query key names in declaration order. */
    get keys(): readonly string[] {
        return Object.keys(this.#definition.keys.shape);
    }

    /** The key naming the person whose key seals the data's personal values, absent for a kind without. */
    get subject(): string | undefined {
        return this.#definition.subject;
    }

    /** Replace each value the data schema marks sensitive by what `map` makes of it, leaving out a value it maps to nothing. */
    mapSensitive(
        data: JsonValue,
        map: (value: JsonValue, sensitivity: schema.Sensitivity) => JsonValue | undefined,
    ): JsonValue {
        return schema.mapSensitive(this.#definition.data, data, map) ?? null;
    }

    /** The scopes each event is copied to beside its own, absent for a kind kept only where it happens. */
    get route(): Route | undefined {
        return this.#definition.route;
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

    /** The column type of each query key. */
    get keyTypes(): Readonly<Record<string, EventKeyType>> {
        return Object.fromEntries(
            Object.entries(this.#definition.keys.shape).map(([name, key]) => [
                name,
                keyTypeOf(key),
            ]),
        );
    }

    /** Write an event as a row of the kind's table, each query key a column. */
    row(event: Event): InsertValue<ReturnType<typeof tableOf>> {
        return {
            ...this.keyValues(event.keys),
            scope: event.scope,
            id: event.id,
            source: event.source,
            time: event.time,
            data: event.data,
        };
    }

    /** Read a row of the kind's table as the event it keeps. */
    event(row: Readonly<Record<string, unknown>>): Event {
        // require the columns every event has
        const { scope, id, source, time, data } = row;
        if (
            typeof scope !== "string" ||
            typeof id !== "string" ||
            typeof source !== "string" ||
            typeof time !== "number"
        ) {
            throw new TypeError(`a ${this.key} row has no scope, identity, source or time`);
        }

        // read each query key's column
        return {
            scope,
            id,
            source,
            time,
            keys: this.keyValues(row),
            data: schema.json().parse(data),
        };
    }

    /** Read the query keys' values from a record holding them, null where absent. */
    keyValues(
        values: Readonly<Record<string, unknown>>,
    ): Readonly<Record<string, string | number | null>> {
        return Object.fromEntries(
            this.keys.map((name) => {
                const value = values[name];

                return [
                    name,
                    typeof value === "string" || typeof value === "number" ? value : null,
                ];
            }),
        );
    }
}

/** Declare an event kind and the append-only table its hot events live in, refusing an invalid name, key, policy or subject. */
export function defineEventKind<
    Shape extends Readonly<Record<string, schema.Schema<string | number | null>>>,
    Data extends JsonValue,
>(
    definition: EventKindDefinition<Shape, Data> & {
        readonly subject?: keyof Shape & string;
    },
    module?: ModuleMetadata,
): EventKind<Shape, Data> {
    // stamp the declaring package and check the name and policy
    const metadata = ModuleMetadata.require(module, "defineEventKind");
    const owner = Package.parse(metadata.package);
    if (!KIND_NAME.test(definition.name)) {
        throw new TypeError(`event kind name must be kebab-case: ${definition.name}`);
    }
    FlushPolicy.parse(definition.policy.flush);
    schema.number().int().positive().parse(definition.policy.retention);

    // require keys of a column type, none named after a column every event has
    for (const [name, key] of Object.entries(definition.keys.shape)) {
        if (RESERVED_KEYS.has(name)) {
            throw new TypeError(
                `event kind ${definition.name} names a key ${name} every event has`,
            );
        }
        keyTypeOf(key);
    }

    // require the subject to name a text key
    const subject =
        definition.subject === undefined ? undefined : definition.keys.shape[definition.subject];
    if (
        definition.subject !== undefined &&
        (subject === undefined || keyTypeOf(subject) !== "text")
    ) {
        throw new TypeError(`event kind ${definition.name} names its subject by a text key`);
    }

    return new EventKind(owner, definition, tableOf(definition, metadata));
}

/** Read the column type of a query key from the JSON type its schema describes. */
function keyTypeOf(key: schema.Schema): EventKeyType {
    // read the described type, ignoring null
    const described = toJsonSchema(key);
    const types = (Array.isArray(described.type) ? described.type : [described.type]).filter(
        (type) => type !== "null",
    );

    // map text, whole numbers and numbers to their columns
    const [type] = types;
    if (types.length !== 1) {
        throw new TypeError(`an event key holds one type of value: ${JSON.stringify(described)}`);
    } else if (type === "string") {
        return "text";
    } else if (type === "integer") {
        return "integer";
    } else if (type === "number") {
        return "real";
    } else {
        throw new TypeError(`an event key holds text or a number: ${JSON.stringify(described)}`);
    }
}

/** Define the append-only table of a kind's hot events: their scope, identity, source, time, query keys and data. */
function tableOf<Shape extends EventKeyShape, Data extends JsonValue>(
    definition: EventKindDefinition<Shape, Data>,
    module: ModuleMetadata,
) {
    // lay out one indexed column per query key beside the columns every event has
    const keys: Record<string, ColumnBuilder> = Object.fromEntries(
        Object.entries(definition.keys.shape).map(([name, key]) => [
            name,
            columnOf(name, keyTypeOf(key)),
        ]),
    );
    const name = `event_${definition.name.replaceAll("-", "_")}`;
    const table = defineTable(
        name,
        {
            /** The scope keeping the event. */
            scope: text("scope").primaryKey(),
            /** The scope the event happened in. */
            source: text("source").primaryKey(),
            /** The event's identity within its source. */
            id: text("id").primaryKey(),
            /** When the event happened, in Unix microseconds. */
            time: integer("time").notNull(),
            /** The data, its personal values sealed. */
            data: json("data", schema.json()).notNull(),
            ...keys,
        },
        {
            log: { appendOnly: true },
            constraints: (columns): readonly ReturnType<typeof index>[] => [
                index(`${name}_time`).on(columns.scope, columns.time, columns.id),
                ...Object.keys(definition.keys.shape).map((key): ReturnType<typeof index> =>
                    index(`${name}_${snakeOf(key)}`).on(
                        columns.scope,
                        table[TABLE].column(key),
                        columns.time,
                    ),
                ),
            ],
        },
        module,
    );

    return table;
}

/** Build the column of a query key in its type. */
function columnOf(name: string, type: EventKeyType): ColumnBuilder {
    const column = snakeOf(name);

    return type === "text" ? text(column) : type === "integer" ? integer(column) : real(column);
}

/** Write a camel-case key name in snake case, as its column is named. */
function snakeOf(name: string): string {
    return name.replaceAll(/[A-Z]/gu, (letter) => `_${letter.toLowerCase()}`);
}
