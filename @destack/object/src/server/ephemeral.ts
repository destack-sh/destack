import { AccessContext, Caller } from "@destack/access";
import { found, schema } from "@destack/schema";
import {
    Change,
    Key,
    TABLE,
    Table,
    type DatabaseConnection,
    type Tracker,
    type TrackerEvent,
} from "@destack/db";
import * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";

/** The tracker topic the ephemeral storages of a database replicate their rows on. */
const ROWS_TOPIC = "ephemeral";

/** How long an untracked owner's rows stay, in milliseconds: 10 s spans two 1–3 s reconnects. */
const LINGER_MILLISECONDS = 10_000;

/** A row's name: its table's SQL name followed by each key value in JSON form. */
const KeyName = schema.tuple([schema.string()], schema.json());

/** A change of one ephemeral row between instances. */
const EphemeralChange = schema.object({
    /** The table's SQL name. */
    table: schema.string(),
    /** The row's name: its table and key as JSON. */
    key: schema.string(),
    /** The row's values in JSON form, null once deleted. */
    row: schema.jsonObject().nullable(),
    /** The owner whose end removes the row. */
    owner: schema.string(),
});
/** A change of one ephemeral row between instances. */
type EphemeralChange = schema.Infer<typeof EphemeralChange>;

/** A message between the ephemeral storages of a database's instances. */
const EphemeralMessage = schema.discriminatedUnion("kind", [
    schema.object({ kind: schema.literal("write"), rows: schema.array(EphemeralChange) }),
    schema.object({ kind: schema.literal("end"), owner: schema.string() }),
    schema.object({ kind: schema.literal("hello") }),
    schema.object({ kind: schema.literal("state"), rows: schema.array(EphemeralChange) }),
]);
/** A message between the ephemeral storages of a database's instances. */
type EphemeralMessage = schema.Infer<typeof EphemeralMessage>;

/** How ephemeral storage replicates: its failure report and the linger. */
export interface EphemeralStorageOptions {
    /** Report a failure no call answers. */
    readonly report: (error: unknown) => void;
    /** How long an untracked owner's rows stay, in milliseconds, 10 s by default. */
    readonly linger?: number;
}

/** The ephemeral objects one instance serves from memory, after Phoenix.Presence. */
export class EphemeralStorage implements AsyncDisposable {
    /** The memory database with the ephemeral objects' tables. */
    readonly database: DatabaseConnection;
    /** The ephemeral object types stored. */
    readonly objects: readonly ObjectType[];
    /** The tracker of the live instances and the owners they track. */
    readonly tracker: Tracker;
    /** The feed serving every sync of the ephemeral objects. */
    readonly feed: sync.Feed;
    /** The ephemeral tables by SQL name. */
    readonly #tables: ReadonlyMap<string, Table>;
    /** The ephemeral object types by table. */
    readonly #byTable: ReadonlyMap<Table, ObjectType>;
    /** Each owner's rows and their last writing instance. */
    readonly #owners = new Map<string, { instance: string; keys: Set<string> }>();
    /** The owner of each row, by row name. */
    readonly #rows = new Map<string, string>();
    /** Report a failure to apply or end rows. */
    readonly #report: (error: unknown) => void;
    /** Stops the pending ends of owners no instance tracks, by owner. */
    readonly #leaving = new Map<string, AbortController>();
    /** Stop listening to the tracker. */
    readonly #unwatch: readonly (() => void)[];
    /** How long an untracked owner's rows stay, in milliseconds. */
    readonly #linger: number;
    /** The pending applications of other instances' messages, in order. */
    #applied = Promise.resolve();
    /** The memory database this store opened and closes, absent if its caller has one. */
    #memory: { readonly close: () => Promise<void> } | undefined;

    /** Store ephemeral object types in memory, replicated over a tracker of live instances. */
    constructor(
        database: DatabaseConnection,
        objects: readonly ObjectType[],
        tracker: Tracker,
        options: EphemeralStorageOptions,
    ) {
        // require ephemeral objects logging every column and referencing only each other
        const names = new Set(objects.map((object) => object.table[TABLE].sqlName));
        for (const object of objects) {
            const { columns, logged } = object.table[TABLE];
            const unlogged = Object.keys(columns).find(
                (property) => !Object.hasOwn(logged, property),
            );
            if (object.storage !== "ephemeral") {
                throw new TypeError(`object ${object.name} is durable, not ephemeral`);
            } else if (unlogged !== undefined) {
                throw new TypeError(`ephemeral object ${object.name} does not log ${unlogged}`);
            }
            for (const constraint of object.table[TABLE].constraints(database.dialect)) {
                const [foreign] = constraint.kind === "foreignKey" ? constraint.foreignColumns : [];
                if (foreign !== undefined && !names.has(foreign.table)) {
                    throw new TypeError(
                        `ephemeral object ${object.name} references ${foreign.table} in another database`,
                    );
                }
            }
        }

        // serve the tables' changes
        const tables = objects.map((object) => object.table);
        this.database = database;
        this.objects = objects;
        this.tracker = tracker;
        this.#tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
        this.#byTable = new Map(objects.map((object) => [object.table, object]));
        this.#report = options.report;
        this.#linger = options.linger ?? LINGER_MILLISECONDS;
        this.feed = new sync.Feed(database, tables);

        // apply the other instances' rows in order, end their owners' rows, and drop stopped instances' rows
        this.#unwatch = [
            tracker.listen(ROWS_TOPIC, (event) => {
                this.#applied = this.#applied.then(() => this.#receive(event)).catch(this.#report);
            }),
            tracker.watchTracked((owner, isTracked) =>
                isTracked ? this.#stay(owner) : this.#leave(owner),
            ),
            tracker.watchStopped((instance) => {
                this.#applied = this.#applied.then(() => this.#drop(instance)).catch(this.#report);
            }),
        ];

        // ask the instances already running for their rows
        this.#send({ kind: "hello" });
    }

    /** Store ephemeral types in a migrated memory database, replicated over a tracker. */
    static async open(
        objects: readonly ObjectType[],
        tracker: Tracker,
        open: (
            tables: readonly Table[],
        ) => Promise<DatabaseConnection & { readonly close: () => Promise<void> }>,
        options: EphemeralStorageOptions,
    ): Promise<EphemeralStorage | undefined> {
        // store nothing for objects without ephemeral types
        const ephemeral = objects.filter((object) => object.storage === "ephemeral");
        if (ephemeral.length === 0) {
            return undefined;
        }

        // migrate their tables in a separate database
        const tables = ephemeral.map((object) => object.table);
        const database = await open(tables);
        await database.migrate(tables);

        // replicate them over the tracker, closing the memory with the store
        const storage = new EphemeralStorage(database, ephemeral, tracker, options);
        storage.#memory = database;

        return storage;
    }

    /** Build the owner of a writer's rows of one object type. */
    static owner(writer: string, object: ObjectType): string {
        return `${EphemeralStorage.#typeKey(object)}/${writer}`;
    }

    /** Key a writer by its caller and identifier. */
    static writerKey(caller: AccessContext, writerId: string): string {
        return JSON.stringify([Caller.key(caller), writerId]);
    }

    /** Write in a memory transaction as a writer and publish its rows. */
    async write<Value>(
        writer: string,
        run: (transaction: DatabaseConnection) => Promise<Value>,
    ): Promise<Value> {
        // record the last image of each written row under its writer's owner
        const { value, rows } = await this.database.transaction(async (transaction) => {
            // run the writer's work, then read its changes
            const written = await run(transaction);
            const tables = [...this.#tables.values()];
            const images = new Map<string, EphemeralChange>();
            for (const change of (await transaction.log.readTransaction({ tables })).changes) {
                const key = Key.name(change.table, change.key);
                const after = Change.after(change);
                images.set(key, {
                    table: change.table[TABLE].sqlName,
                    key,
                    row: after === null ? null : change.table[TABLE].encode(after),
                    owner: EphemeralStorage.owner(writer, found(this.#byTable, change.table)),
                });
            }

            return { value: written, rows: [...images.values()] };
        });

        // publish them, and schedule the ends of untracked owners
        if (rows.length > 0) {
            for (const row of rows) {
                this.#storeChange(row, this.tracker.instance);
            }
            this.#send({ kind: "write", rows });
        }
        for (const owner of new Set(rows.map((row) => row.owner))) {
            if (!this.tracker.isTracked(owner)) {
                this.#leave(owner);
            }
        }

        return value;
    }

    /** Track a writer's rows while one of its streams is open, returning the release. */
    track(writer: string): () => void {
        for (const object of this.objects) {
            this.tracker.track(EphemeralStorage.owner(writer, object));
        }

        return () => {
            for (const object of this.objects) {
                this.tracker.release(EphemeralStorage.owner(writer, object));
            }
        };
    }

    /** Wait until every received message is applied. */
    settled(): Promise<void> {
        return this.#applied;
    }

    /** Stop replicating, ending every pending linger at once. */
    close(): void {
        for (const unwatch of this.#unwatch) {
            unwatch();
        }
        for (const leaving of this.#leaving.values()) {
            leaving.abort();
        }
        this.#leaving.clear();
    }

    /** Stop replicating, then close the memory database this store opened. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.close();
        await this.#memory?.close();
    }

    /** Apply another instance's message. */
    async #receive({ instance, event }: TrackerEvent): Promise<void> {
        // skip this instance's own messages
        if (instance === this.tracker.instance) {
            return;
        }

        // apply rows, end owners, and answer a newcomer with this instance's rows
        const message = EphemeralMessage.parse(event);
        if (message.kind === "write" || message.kind === "state") {
            await this.#apply(message.rows, instance);
        } else if (message.kind === "end") {
            await this.#end(message.owner);
        } else {
            this.#send({ kind: "state", rows: await this.#state() });
        }
    }

    /** Send a message to the other instances' storages. */
    #send(message: EphemeralMessage): void {
        this.tracker.broadcast(ROWS_TOPIC, message);
    }

    /** Write another instance's rows into this instance's tables. */
    async #apply(rows: readonly EphemeralChange[], instance: string): Promise<void> {
        await this.database.transaction(async (transaction) => {
            for (const change of rows) {
                // upsert a live row and remove a deleted one
                const table = found(this.#tables, change.table);
                if (change.row === null) {
                    await transaction
                        .delete(table)
                        .where(Key.match(table, Key.parse(table, change.key)));
                } else {
                    await transaction.upsert(table, [Table.decode(table, change.row)]);
                }
            }
        });
        for (const change of rows) {
            this.#storeChange(change, instance);
        }
    }

    /** Store a row under its owner, or forget it once deleted. */
    #storeChange(change: EphemeralChange, instance: string): void {
        // move the row from a previous owner
        const previous = this.#rows.get(change.key);
        if (previous !== undefined && previous !== change.owner) {
            this.#forget(previous, change.key);
        }

        // forget a deleted row or store a live one
        if (change.row === null) {
            this.#forget(change.owner, change.key);
        } else {
            const entry = this.#owners.get(change.owner) ?? { instance, keys: new Set<string>() };
            entry.instance = instance;
            entry.keys.add(change.key);
            this.#owners.set(change.owner, entry);
            this.#rows.set(change.key, change.owner);
        }
    }

    /** Forget one row of an owner, and the owner once it has none. */
    #forget(owner: string, key: string): void {
        // forget the row, and the owner without rows
        const entry = this.#owners.get(owner);
        entry?.keys.delete(key);
        this.#rows.delete(key);
        if (entry?.keys.size === 0) {
            this.#owners.delete(owner);
        }
    }

    /** Remove an owner's rows from this instance's tables. */
    async #end(owner: string): Promise<void> {
        // forget the owner's rows
        const entry = this.#owners.get(owner);
        if (entry === undefined) {
            return;
        }
        this.#owners.delete(owner);
        for (const key of entry.keys) {
            this.#rows.delete(key);
        }

        // delete them, one statement per table
        const byTable = Map.groupBy(entry.keys, (key) => EphemeralStorage.#tableName(key));
        await this.database.transaction(async (transaction) => {
            for (const [name, keys] of byTable) {
                const table = found(this.#tables, name);
                await transaction.remove(
                    table,
                    keys.map((key) => Key.parse(table, key)),
                );
            }
        });
    }

    /** Remove the rows a stopped instance last wrote. */
    async #drop(instance: string): Promise<void> {
        for (const [owner, entry] of this.#owners) {
            if (entry.instance === instance) {
                await this.#end(owner);
            }
        }
    }

    /** Read the rows this instance last wrote. */
    async #state(): Promise<EphemeralChange[]> {
        const rows: EphemeralChange[] = [];
        for (const [owner, entry] of this.#owners) {
            if (entry.instance !== this.tracker.instance) {
                continue;
            }
            for (const key of entry.keys) {
                const table = found(this.#tables, EphemeralStorage.#tableName(key));
                const [row] = await this.database
                    .select()
                    .from(table)
                    .where(Key.match(table, Key.parse(table, key)));
                if (row === undefined) {
                    throw new TypeError(`ephemeral row ${key} is missing from its table`);
                }
                rows.push({
                    table: table[TABLE].sqlName,
                    key,
                    row: table[TABLE].encode(row),
                    owner,
                });
            }
        }

        return rows;
    }

    /** End an untracked owner's rows everywhere after the linger on the feed's clock. */
    #leave(owner: string): void {
        // replace a pending end
        this.#leaving.get(owner)?.abort();
        const leaving = new AbortController();
        this.#leaving.set(owner, leaving);

        // end the rows once the linger passes untracked
        this.feed.clock.sleep(this.#linger, leaving.signal).then(
            () => {
                this.#leaving.delete(owner);
                if (!this.tracker.isTracked(owner)) {
                    this.#send({ kind: "end", owner });
                    this.#end(owner).catch(this.#report);
                }
            },
            () => {},
        );
    }

    /** Cancel the pending end of an owner some instance tracks again. */
    #stay(owner: string): void {
        this.#leaving.get(owner)?.abort();
        this.#leaving.delete(owner);
    }

    /** Build the key of an object type within its rows' owners. */
    static #typeKey(object: ObjectType): string {
        return `${object.policy.definition.packageId}/${object.name}`;
    }

    /** Read the SQL name of the table a row's name starts with. */
    static #tableName(key: string): string {
        const [table] = KeyName.parse(JSON.parse(key));

        return table;
    }
}
