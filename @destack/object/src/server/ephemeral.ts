import { Caller } from "@destack/access";
import { found } from "@destack/schema";
import { TABLE, type DatabaseConnection, type Table, type Channel } from "@destack/db";
import * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";

/** The channel name of the ephemeral objects' changes on their durable database. */
const EPHEMERAL_CHANNEL = "ephemeral";

/** How long an untracked owner's rows stay, in milliseconds: 10 s spans two 1–3 s reconnects. */
const LINGER_MILLISECONDS = 10_000;

/** How ephemeral storage replicates: the tracker's options, its failure report and the linger. */
export interface EphemeralStorageOptions extends sync.TrackerOptions {
    /** Report a failure no call answers. */
    readonly report: (error: unknown) => void;
    /** How long an untracked owner's rows stay, in milliseconds, 10 s by default. */
    readonly linger?: number;
}

/** The ephemeral objects one instance serves from a replicated memory database. */
export class EphemeralStorage implements AsyncDisposable {
    /** The memory database with the ephemeral objects' tables. */
    readonly database: DatabaseConnection;
    /** The ephemeral object types kept. */
    readonly objects: readonly ObjectType[];
    /** The replication of rows and tracked clients between instances. */
    readonly tracker: sync.Tracker;
    /** The feed serving every sync of the ephemeral objects. */
    readonly feed: sync.Feed;
    /** The open ephemeral streams of each writer on this instance. */
    readonly #streams = new Map<string, number>();
    /** The ephemeral object types by table. */
    readonly #byTable: ReadonlyMap<Table, ObjectType>;
    /** Report a failure to end an owner's rows. */
    readonly #report: (error: unknown) => void;
    /** The pending ends of owners no instance tracks, by owner. */
    readonly #leaving = new Map<string, ReturnType<typeof setTimeout>>();
    /** Stop watching which owners any instance tracks. */
    readonly #unwatch: () => void;
    /** How long an untracked owner's rows stay, in milliseconds. */
    readonly #linger: number;
    /** The memory database this store opened and closes, absent if its caller keeps one. */
    #owned: { close(): Promise<void> } | undefined;

    /** Keep ephemeral object types in memory, replicated over a channel. */
    constructor(
        database: DatabaseConnection,
        objects: readonly ObjectType[],
        channel: Channel<unknown>,
        options: EphemeralStorageOptions,
    ) {
        // require ephemeral objects referencing only each other
        const names = new Set(objects.map((object) => object.table[TABLE].sqlName));
        for (const object of objects) {
            if (object.storage !== "ephemeral") {
                throw new TypeError(`object ${object.name} is durable, not ephemeral`);
            }
            for (const constraint of object.table[TABLE].constraints(database.dialect)) {
                const [foreign] = constraint.kind === "foreignKey" ? constraint.foreignColumns : [];
                if (foreign !== undefined && !names.has(foreign.table)) {
                    throw new TypeError(
                        `ephemeral object ${object.name} references ${foreign.table}, which lives in another database`,
                    );
                }
            }
        }

        // replicate the tables and serve their changes
        const tables = objects.map((object) => object.table);
        this.database = database;
        this.objects = objects;
        this.#byTable = new Map(objects.map((object) => [object.table, object]));
        this.#report = options.report;
        this.#linger = options.linger ?? LINGER_MILLISECONDS;
        this.tracker = new sync.Tracker(database, tables, channel, options);
        this.feed = new sync.Feed(database, tables, { clock: options.clock });

        // track owners
        this.#unwatch = this.tracker.watchTracked((owner, isTracked) =>
            isTracked ? this.#stay(owner) : this.#leave(owner),
        );
    }

    /** Keep ephemeral types in a migrated memory database, replicated over a channel. */
    static async open(
        objects: readonly ObjectType[],
        channel: Channel<unknown>,
        open: (
            tables: readonly Table[],
        ) => Promise<DatabaseConnection & { close(): Promise<void> }>,
        options: EphemeralStorageOptions,
    ): Promise<EphemeralStorage | undefined> {
        // keep nothing for objects without ephemeral types
        const ephemeral = objects.filter((object) => object.storage === "ephemeral");
        if (ephemeral.length === 0) {
            return undefined;
        }

        // migrate their tables in a database of their own
        const tables = ephemeral.map((object) => object.table);
        const database = await open(tables);
        await database.migrate(tables);

        // replicate them over the channel, closing the memory with the store
        const storage = new EphemeralStorage(database, ephemeral, channel, options);
        storage.#owned = database;

        return storage;
    }

    /** Open the channel replicating ephemeral objects across a durable database's connections. */
    static channel(durable: DatabaseConnection): Channel<unknown> {
        return durable.channel(EPHEMERAL_CHANNEL);
    }

    /** Build the owner of a writer's rows of one object type. */
    static owner(writer: string, object: ObjectType): string {
        return `${typeKey(object)}/${writer}`;
    }

    /** Write in a memory transaction as a writer and publish its rows. */
    async write<Value>(
        writer: string,
        run: (transaction: DatabaseConnection) => Promise<Value>,
    ): Promise<Value> {
        // record the written rows
        const { value, rows } = await this.database.transaction(async (transaction) => {
            const written = await run(transaction);
            const recorded = await this.tracker.record(transaction, (table) =>
                EphemeralStorage.owner(writer, found(this.#byTable, table)),
            );

            return { value: written, rows: recorded };
        });

        // publish them and schedule untracked owners' ends
        this.tracker.publish(rows);
        for (const owner of new Set(rows.map((row) => row.owner))) {
            if (!this.tracker.isTracked(owner)) {
                this.#leave(owner);
            }
        }

        return value;
    }

    /** Key a writer by its caller and identifier. */
    static writerKey(caller: Caller, writerId: string): string {
        return JSON.stringify([Caller.key(caller), writerId]);
    }

    /** Track a writer's rows while one of its streams is open, returning the release. */
    track(writer: string): () => void {
        // track the writer's owners on the first stream
        const streams = this.#streams.get(writer) ?? 0;
        this.#streams.set(writer, streams + 1);
        if (streams === 0) {
            for (const object of this.objects) {
                this.tracker.track(EphemeralStorage.owner(writer, object));
            }
        }

        return () => {
            // release them on the last stream
            const left = found(this.#streams, writer) - 1;
            if (left > 0) {
                this.#streams.set(writer, left);

                return;
            }
            this.#streams.delete(writer);
            for (const object of this.objects) {
                this.tracker.release(EphemeralStorage.owner(writer, object));
            }
        };
    }

    /** Stop replicating, ending every pending linger at once. */
    close(): void {
        // stop watching and clear pending ends
        this.#unwatch();
        for (const timer of this.#leaving.values()) {
            clearTimeout(timer);
        }
        this.#leaving.clear();
        this.tracker.close();
    }

    /** Stop replicating and close the memory database this store opened. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.close();
        await this.#owned?.close();
    }

    /** End an untracked owner's rows after the linger. */
    #leave(owner: string): void {
        // schedule the end
        clearTimeout(this.#leaving.get(owner));
        this.#leaving.set(
            owner,
            setTimeout(() => {
                this.#leaving.delete(owner);
                if (!this.tracker.isTracked(owner)) {
                    this.tracker.end(owner).catch(this.#report);
                }
            }, this.#linger),
        );
    }

    /** Cancel the pending end of an owner some instance tracks again. */
    #stay(owner: string): void {
        clearTimeout(this.#leaving.get(owner));
        this.#leaving.delete(owner);
    }
}

/** Build the key of an object type within its rows' owners. */
function typeKey(object: ObjectType): string {
    return `${object.policy.definition.packageId}/${object.name}`;
}
