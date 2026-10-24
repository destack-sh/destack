import { TestTelemetry } from "@destack/telemetry/test";
import {
    ControlLoop,
    type ColumnBuilder,
    defineTable,
    index,
    integer,
    json,
    real,
    text,
} from "@destack/db";
import { present, schema, toJsonSchema, type JsonValue } from "@destack/schema";
import { TestDatabase } from "@destack/db/test";
import { LocalKeyring } from "@destack/identity";
import { MemoryBucket } from "@destack/bucket/test";
import { DatabasePersonalKeyring } from "../store/personal-keyring.ts";
import {
    EventKind,
    type EventKeyFields,
    type EventKeyType,
    type EventKindDescription,
    type SettingRetention,
    eventColumns,
} from "../kind/kind.ts";
import { eventTables } from "../stack/db.ts";
import { EventStore } from "../store/store.ts";
import { Simulation, type SimulatedClock } from "@destack/environment/simulation";

/** When every fixture's clock starts: 1 October 2026, UTC. */
const START = Date.UTC(2026, 9, 1);

/** Build a test kind and its table: the event columns, and an indexed column per key. */
export function testKind<Fields extends EventKeyFields, Data extends JsonValue>(
    description: Omit<EventKindDescription<Fields, Data>, "table">,
    search: readonly string[] = [],
): EventKind<Fields, Data> {
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
                        : json(column, schema.jsonObject());

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
    /** The test database that stores the hot events, the catalog and people's keys. */
    readonly database: TestDatabase;
    /** The clock the database, the controllers and the bucket's locks read, which tests set. */
    readonly clock: SimulatedClock;
    /** The memory bucket that stores the segments, its locks on the fixture's clock. */
    readonly bucket: MemoryBucket;
    /** The telemetry that records the exceptions the store captures. */
    readonly telemetry: TestTelemetry;
    /** The stop of the store's control loop. */
    readonly #stop = new AbortController();
    /** The failures the store's controllers reported. */
    readonly #failures: unknown[] = [];
    /** The store's control loop and its run, started at the first settle. */
    #controlled: { readonly loop: ControlLoop; readonly running: Promise<void> } | undefined;

    /** Store opened events. */
    private constructor(
        open: (fixture: EventFixture) => EventStore,
        database: TestDatabase,
        clock: SimulatedClock,
        telemetry: TestTelemetry,
    ) {
        // store the database, clock and telemetry, then open the store on the fixture's bucket
        this.database = database;
        this.clock = clock;
        this.telemetry = telemetry;
        this.bucket = new MemoryBucket(clock);
        this.store = open(this);
    }

    /** Open events of some kinds over a fresh database, memory bucket and new keyring. */
    static async open(
        kinds: readonly EventKind[],
        settings?: SettingRetention,
    ): Promise<EventFixture> {
        // migrate the kinds' tables on the fixture's clock
        const tables = eventTables(kinds);
        const simulation = new Simulation({ seed: 1, start: START });
        const environment = simulation.environment("events");
        const { clock } = simulation;
        const database = await TestDatabase.create(tables, { environment });
        await database.database.migrate(tables);
        const keyring = await LocalKeyring.read(LocalKeyring.generate(environment), environment);

        // store every scope's segments in the one bucket, on the fixture's clock
        return new EventFixture(
            (fixture) =>
                new EventStore({
                    database: database.database,
                    kinds,
                    files: () => fixture.bucket,
                    personal: new DatabasePersonalKeyring(database.database, keyring, environment),
                    ...(settings === undefined ? {} : { settings }),
                    clock: fixture.clock,
                }),
            database,
            clock,
            new TestTelemetry(simulation.telemetry),
        );
    }

    /** Run the store's controllers in a control loop until they settle, failing on their errors. */
    async settle(): Promise<void> {
        // start the loop on the fixture's clock once
        if (this.#controlled === undefined) {
            const loop = new ControlLoop(this.store.database, this.store.controllers, {
                clock: this.clock,
                report: (_controller, _key, error) => this.#failures.push(error),
            });
            const running = loop.run(this.#stop.signal).catch((error: unknown) => {
                this.#failures.push(error);
            });
            this.#controlled = { loop, running };
        }

        // wait until nothing is due, failing on the first reported error
        await this.#controlled.loop.idle();
        if (this.#failures.length > 0) {
            throw this.#failures[0];
        }
    }

    /** Stop the control loop, then close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        // stop the loop, then close the database and the telemetry
        this.#stop.abort();
        await this.#controlled?.running;
        await this.database.close();
        this.telemetry[Symbol.dispose]();
    }
}

/** Read the column type of a test key's values, from its schema's JSON type. */
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
