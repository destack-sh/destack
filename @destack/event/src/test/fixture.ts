import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LocalBucket } from "@destack/bucket/local";
import type { Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { LocalKeyring } from "@destack/identity";
import { Outbox, outbox } from "@destack/service/outbox";
import { DatabasePersonalKeyring } from "../personal/personal.ts";
import type { Event, EventKind, EventPolicy, Route } from "../kind/kind.ts";
import { eventTables } from "../stack/db.ts";
import { EventStore } from "../store/store.ts";

/** What a test keeps events with beside its kinds: a scope's policy and the delivery of routed copies. */
export interface EventFixtureOptions {
    /** Read a scope's policy for a kind, the kind's own when absent. */
    readonly policy?: (kind: EventKind, scope: string) => Promise<EventPolicy | undefined>;
    /** Name the scopes a route copies a scope's events to, none when absent. */
    readonly targets?: (route: Route, scope: string) => Promise<readonly string[]>;
    /** Deliver routed copies, such as to another fixture's store. */
    readonly deliver?: (
        kind: string,
        events: readonly Event[],
        signal: AbortSignal,
    ) => Promise<void>;
}

/** A host's events over a test database, a local bucket and a keyring, on a clock the test sets, for tests. */
export class EventFixture implements AsyncDisposable {
    /** The store. */
    readonly store: EventStore;
    /** The test database keeping the hot events, the catalog, people's keys and the outbox. */
    readonly database: TestDatabase;
    /** The local bucket keeping the segments. */
    readonly bucket: LocalBucket;
    /** The current time the controllers read, in Unix milliseconds. */
    now = Date.UTC(2026, 9, 1);
    /** The bucket's directory. */
    readonly #directory: string;

    /** Hold opened events. */
    private constructor(
        open: (fixture: EventFixture) => EventStore,
        database: TestDatabase,
        bucket: LocalBucket,
        directory: string,
    ) {
        // hold what the store keeps its events in, then open it on the fixture's clock
        this.database = database;
        this.bucket = bucket;
        this.#directory = directory;
        this.store = open(this);
    }

    /** Open events of some kinds over a fresh database of a dialect, a temporary bucket and a generated keyring. */
    static async open(
        dialect: Dialect,
        kinds: readonly EventKind[],
        options: EventFixtureOptions = {},
    ): Promise<EventFixture> {
        // migrate the kinds' tables with the outbox, and open a bucket in a temporary directory
        const tables = [...eventTables(kinds), outbox];
        const database = await TestDatabase.create(dialect, tables);
        await database.database.migrate(tables);
        const directory = await mkdtemp(join(tmpdir(), "event-"));
        const bucket = await LocalBucket.open(directory, "events");
        const keyring = await LocalKeyring.read(LocalKeyring.generate());

        // keep every scope's segments in the one bucket, on the fixture's clock
        return new EventFixture(
            (fixture) =>
                new EventStore({
                    database: database.database,
                    kinds,
                    files: () => bucket,
                    personal: new DatabasePersonalKeyring(database.database, keyring),
                    outbox: new Outbox(database.database),
                    targets: options.targets ?? (() => Promise.resolve([])),
                    deliver: options.deliver ?? (() => Promise.resolve()),
                    ...(options.policy === undefined ? {} : { policy: options.policy }),
                    now: () => fixture.now,
                }),
            database,
            bucket,
            directory,
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

    /** Close the database and remove the bucket. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.bucket[Symbol.asyncDispose]();
        await this.database.close();
        await rm(this.#directory, { recursive: true, force: true });
    }
}
