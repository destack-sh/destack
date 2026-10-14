import { TestTelemetry } from "@destack/telemetry/test";
import { TestClock } from "@destack/test";
import {
    type ColumnBuilder,
    defineTable,
    index,
    integer,
    json,
    real,
    text,
    type Dialect,
} from "@destack/db";
import { present, schema, toJsonSchema, type JsonValue } from "@destack/schema";
import { TestDatabase } from "@destack/db/test";
import { LocalKeyring } from "@destack/identity";
import { MemoryBucket } from "@destack/bucket/test";
import { DatabasePersonalKeyring } from "../store/personal-keyring.ts";
import {
    EventKind,
    type EventKeyShape,
    type EventKeyType,
    type EventKindDescription,
    type EventPolicy,
    eventColumns,
} from "../kind/kind.ts";
import { eventTables } from "../stack/db.ts";
import { EventStore } from "../store/store.ts";

/** The test event options beside kinds: a scope's policy. */
export interface EventFixtureOptions {
    /** Read a scope's policy for a kind, the kind's own when absent. */
    readonly policy?: (kind: EventKind, scope: string) => Promise<EventPolicy>;
}

/** Build a test kind and its table: the event columns, and an indexed column per key. */
export function testKind<Shape extends EventKeyShape, Data extends JsonValue>(
    description: Omit<EventKindDescription<Shape, Data>, "table">,
    search: readonly string[] = [],
): EventKind<Shape, Data> {
    // lay out a column per key beside the event columns
    const name = `event_${description.name.replaceAll("-", "_")}`;
    const keys = Object.entries(description.keys.shape);

    // build each key's column by its type, a map of entries as JSON
    const columns = Object.fromEntries(
        keys.map(([key, value]) => {
            // declare the column of the key's type under its snake-case name
            const column = snake(key);
            const type = readKeyType(value);
            const built =
                type === "text"
                    ? text(column)
                    : type === "integer"
                      ? integer(column)
                      : type === "real"
                        ? real(column)
                        : json(column, schema.record(schema.string(), schema.json()));

            return [key, built];
        }),
    );
    const builders: ReturnType<typeof eventColumns> & Readonly<Record<string, ColumnBuilder>> = {
        ...columns,
        ...eventColumns(),
    };
    const table = defineTable(
        name,
        builders,
        {
            log: { appendOnly: true },
            fullText: search,
            constraints: (built) => [
                index(`${name}_time`).on(built.scope, built.time, built.id),
                ...keys
                    .filter(([, value]) => readKeyType(value) !== "json")
                    .map(([key]) =>
                        index(`${name}_${snake(key)}`).on(
                            built.scope,
                            present(built[key], `the column of ${key}`),
                            built.time,
                        ),
                    ),
            ],
        },
        { package: description.package },
    );

    return new EventKind({ ...description, table });
}

/** Write a camel-case key in snake case, as its column is named. */
function snake(key: string): string {
    return key.replaceAll(/[A-Z]/gu, (letter) => `_${letter.toLowerCase()}`);
}

/** A host's events over a test database, memory bucket and keyring, on a clock the test sets. */
export class EventFixture implements AsyncDisposable {
    /** The store. */
    readonly store: EventStore;
    /** The test database keeping the hot events, the catalog and people's keys. */
    readonly database: TestDatabase;
    /** The clock the controllers and the bucket's locks read, which tests set. */
    readonly clock = new TestClock(Date.UTC(2026, 9, 1));
    /** The memory bucket keeping the segments, its locks on the fixture's clock. */
    readonly bucket = new MemoryBucket(this.clock);
    /** The telemetry keeping the exceptions the store captures. */
    readonly telemetry = TestTelemetry.register();

    /** Hold opened events. */
    private constructor(open: (fixture: EventFixture) => EventStore, database: TestDatabase) {
        // hold the database, then open the store on the fixture's bucket and clock
        this.database = database;
        this.store = open(this);
    }

    /** Open events of some kinds over a fresh dialect database, memory bucket and new keyring. */
    static async open(
        dialect: Dialect,
        kinds: readonly EventKind[],
        options: EventFixtureOptions = {},
    ): Promise<EventFixture> {
        // migrate the kinds' tables
        const tables = eventTables(kinds);
        const database = await TestDatabase.create(dialect, tables);
        await database.database.migrate(tables);
        const keyring = await LocalKeyring.read(LocalKeyring.generate());

        // keep every scope's segments in the one bucket, on the fixture's clock
        return new EventFixture(
            (fixture) =>
                new EventStore({
                    database: database.database,
                    kinds,
                    files: () => fixture.bucket,
                    personal: new DatabasePersonalKeyring(database.database, keyring),
                    ...(options.policy === undefined ? {} : { policy: options.policy }),
                    clock: fixture.clock,
                }),
            database,
        );
    }

    /** Reconcile every key each controller lists once, as the control loop would. */
    async settle(): Promise<void> {
        const signal = new AbortController().signal;
        for (const controller of this.store.controllers) {
            for (const key of await controller.list()) {
                await controller.reconcile(key, {
                    signal,
                    changed: () =>
                        new Promise<void>((resolve) => {
                            signal.addEventListener("abort", () => {
                                resolve();
                            });
                        }),
                });
            }
        }
    }

    /** Close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.database.close();
        this.telemetry[Symbol.dispose]();
    }
}

/** Read the column type a test key's schema keeps its values in, from its JSON type. */
function readKeyType(key: schema.Schema): EventKeyType {
    // read the described types of the key and each of its alternatives, ignoring null
    const described = toJsonSchema(key);
    const types = [described, ...(described.anyOf ?? [])]
        .flatMap((each) => (typeof each === "object" ? [each.type].flat() : []))
        .filter((type) => type !== undefined && type !== "null");

    // map text, whole numbers, numbers and maps of entries to their columns
    const [type] = types;
    if (types.length !== 1) {
        throw new TypeError(`an event key has one type of value: ${JSON.stringify(described)}`);
    } else if (type === "object") {
        return "json";
    } else if (type === "string") {
        return "text";
    } else if (type === "integer") {
        return "integer";
    } else if (type === "number") {
        return "real";
    } else {
        throw new TypeError(
            `an event key must be text, a number or a map: ${JSON.stringify(described)}`,
        );
    }
}
