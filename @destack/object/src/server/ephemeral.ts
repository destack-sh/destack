import { TABLE, type DatabaseConnection, type Table } from "@destack/db";
import type { Relay } from "@destack/db/relay";
import * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";

/** The ephemeral objects one instance serves from a replicated memory database. */
export class EphemeralStorage {
    /** The memory database holding the ephemeral objects' tables. */
    readonly database: DatabaseConnection;
    /** The ephemeral object types held. */
    readonly objects: readonly ObjectType[];
    /** The replication of rows and client holds between instances. */
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
    /** The pending ends of owners no instance holds, by owner. */
    readonly #leaving = new Map<string, ReturnType<typeof setTimeout>>();
    /** Stop watching which owners any instance holds. */
    readonly #unwatch: () => void;

    /** Hold ephemeral object types in memory, replicated over a relay. */
    constructor(
        database: DatabaseConnection,
        objects: readonly ObjectType[],
        relay: Relay<sync.TrackerMessage>,
        options: sync.TrackerOptions & { readonly report: (error: unknown) => void },
    ) {
        // require ephemeral objects referencing only each other
        const names = new Set(objects.map((object) => object.table[TABLE].sqlName));
        for (const object of objects) {
            if (object.storage !== "ephemeral") {
                throw new TypeError(`object ${object.name} is durable, not ephemeral`);
            }
            for (const constraint of (object.table as Table).constraints(database.dialect)) {
                const foreign =
                    constraint.kind === "foreignKey" ? constraint.foreignColumns[0]! : undefined;
                if (foreign !== undefined && !names.has(foreign.table)) {
                    throw new TypeError(
                        `ephemeral object ${object.name} references ${foreign.table}, which lives in another database`,
                    );
                }
            }
        }

        // replicate the tables and serve their changes
        const tables = objects.map((object) => object.table as Table);
        this.database = database;
        this.objects = objects;
        this.#byTable = new Map(objects.map((object) => [object.table as Table, object]));
        this.#byType = new Map(objects.map((object) => [typeKey(object), object]));
        this.#report = options.report;
        this.tracker = new sync.Tracker(database, tables, relay, options);
        this.feed = new sync.Feed(database, tables);

        // track owner holds
        this.#unwatch = this.tracker.watchHolds((owner, isHeld) =>
            isHeld ? this.#stay(owner) : this.#leave(owner),
        );
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
            const value = await run(transaction);
            const rows = await this.tracker.record(transaction, (table) =>
                EphemeralStorage.owner(client, this.#byTable.get(table)!),
            );

            return { value, rows };
        });

        // publish them and schedule unheld owners' ends
        this.tracker.publish(rows);
        for (const owner of new Set(rows.map((row) => row.owner))) {
            if (!this.tracker.isHeld(owner)) {
                this.#leave(owner);
            }
        }

        return value;
    }

    /** Hold a client's rows while one of its streams is open, returning the release. */
    hold(client: string): () => void {
        // hold the client's owners on the first stream
        const streams = this.#streams.get(client) ?? 0;
        this.#streams.set(client, streams + 1);
        if (streams === 0) {
            for (const object of this.objects) {
                this.tracker.hold(EphemeralStorage.owner(client, object));
            }
        }

        return () => {
            // release them on the last stream
            const left = this.#streams.get(client)! - 1;
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

    /** End an unheld owner's rows after its type's linger. */
    #leave(owner: string): void {
        // find the owner's type
        const object = this.#byType.get(
            owner.slice(0, owner.indexOf("/", owner.indexOf("/") + 1)),
        )!;

        // schedule the end
        clearTimeout(this.#leaving.get(owner));
        this.#leaving.set(
            owner,
            setTimeout(() => {
                this.#leaving.delete(owner);
                if (!this.tracker.isHeld(owner)) {
                    this.tracker.end(owner).catch(this.#report);
                }
            }, object.linger!),
        );
    }

    /** Cancel the pending end of an owner some instance holds again. */
    #stay(owner: string): void {
        clearTimeout(this.#leaving.get(owner));
        this.#leaving.delete(owner);
    }
}

/** Name an object type within its rows' owners. */
function typeKey(object: ObjectType): string {
    return `${object.policy.definition.packageId}/${object.name}`;
}
