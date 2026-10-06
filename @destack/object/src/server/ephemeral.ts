import type { ServiceContext } from "@destack/service/server";
import { found } from "@destack/schema";
import { TABLE, type DatabaseConnection, type Table, type Channel } from "@destack/db";
import * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";

/** The channel name of the ephemeral objects' changes on their durable database. */
const EPHEMERAL_CHANNEL = "ephemeral";

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
    /** The open ephemeral streams of each client on this instance. */
    readonly #streams = new Map<string, number>();
    /** The ephemeral object types by table. */
    readonly #byTable: ReadonlyMap<Table, ObjectType>;
    /** The ephemeral object types by type key. */
    readonly #byType: ReadonlyMap<string, ObjectType>;
    /** Report a failure to end an owner's rows. */
    readonly #report: (error: unknown) => void;
    /** The pending ends of owners no instance tracks, by owner. */
    readonly #leaving = new Map<string, ReturnType<typeof setTimeout>>();
    /** Stop watching which owners any instance tracks. */
    readonly #unwatch: () => void;
    /** The memory database this store opened and closes with itself, absent for one its caller keeps. */
    #owned: { close(): Promise<void> } | undefined;

    /** Keep ephemeral object types in memory, replicated over a channel. */
    constructor(
        database: DatabaseConnection,
        objects: readonly ObjectType[],
        channel: Channel<unknown>,
        options: sync.TrackerOptions & { readonly report: (error: unknown) => void },
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
        this.#byType = new Map(objects.map((object) => [typeKey(object), object]));
        this.#report = options.report;
        this.tracker = new sync.Tracker(database, tables, channel, options);
        this.feed = new sync.Feed(database, tables);

        // track owners
        this.#unwatch = this.tracker.watchTracked((owner, isTracked) =>
            isTracked ? this.#stay(owner) : this.#leave(owner),
        );
    }

    /** Keep the ephemeral types among some objects in a migrated memory database, replicated over a channel. */
    static async open(
        objects: readonly ObjectType[],
        channel: Channel<unknown>,
        open: (
            tables: readonly Table[],
        ) => Promise<DatabaseConnection & { close(): Promise<void> }>,
        options: sync.TrackerOptions & { readonly report: (error: unknown) => void },
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

    /** Open the channel replicating ephemeral objects between the connections of a durable database. */
    static channel(durable: DatabaseConnection): Channel<unknown> {
        return durable.channel(EPHEMERAL_CHANNEL);
    }

    /** Name the owner of a client's rows of one object type. */
    static owner(client: string, object: ObjectType): string {
        return `${typeKey(object)}/${client}`;
    }

    /** Write in a memory transaction as a client and publish its rows. */
    async write<Value>(
        client: string,
        run: (transaction: DatabaseConnection) => Promise<Value>,
    ): Promise<Value> {
        // record the written rows
        const { value, rows } = await this.database.transaction(async (transaction) => {
            const written = await run(transaction);
            const recorded = await this.tracker.record(transaction, (table) =>
                EphemeralStorage.owner(client, found(this.#byTable, table)),
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

    /** Key a client by its caller and identifier. */
    static clientKey(context: ServiceContext, client: string): string {
        return JSON.stringify([context.authentication?.id ?? null, client]);
    }

    /** Track a client's rows while one of its streams is open, returning the release. */
    track(client: string): () => void {
        // track the client's owners on the first stream
        const streams = this.#streams.get(client) ?? 0;
        this.#streams.set(client, streams + 1);
        if (streams === 0) {
            for (const object of this.objects) {
                this.tracker.track(EphemeralStorage.owner(client, object));
            }
        }

        return () => {
            // release them on the last stream
            const left = found(this.#streams, client) - 1;
            if (left > 0) {
                this.#streams.set(client, left);

                return;
            }
            this.#streams.delete(client);
            for (const object of this.objects) {
                this.tracker.release(EphemeralStorage.owner(client, object));
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

    /** End an untracked owner's rows after its type's linger. */
    #leave(owner: string): void {
        // find the owner's type
        const object = found(
            this.#byType,
            owner.slice(0, owner.indexOf("/", owner.indexOf("/") + 1)),
        );

        // schedule the end
        clearTimeout(this.#leaving.get(owner));
        this.#leaving.set(
            owner,
            setTimeout(() => {
                this.#leaving.delete(owner);
                if (!this.tracker.isTracked(owner)) {
                    this.tracker.end(owner).catch(this.#report);
                }
            }, object.linger),
        );
    }

    /** Cancel the pending end of an owner some instance tracks again. */
    #stay(owner: string): void {
        clearTimeout(this.#leaving.get(owner));
        this.#leaving.delete(owner);
    }
}

/** Name an object type within its rows' owners. */
function typeKey(object: ObjectType): string {
    return `${object.policy.definition.packageId}/${object.name}`;
}
