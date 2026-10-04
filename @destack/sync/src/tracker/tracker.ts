import { schema, type JsonValue } from "@destack/schema";
import {
    Change,
    Key,
    or,
    TABLE,
    type DatabaseConnection,
    type Table,
    CHAIN_TERMS,
    type Channel,
    typedChannel,
} from "@destack/db";

/**
 * The heartbeat interval, in milliseconds.
 *
 * Three missed beats bound a crashed instance's rows to 45 s.
 */
const HEARTBEAT_MILLISECONDS = 15_000;

/** The missed heartbeats before the others drop an instance's rows. */
const MISSED_HEARTBEATS = 3;

/** A row's name: its table's SQL name, then each key value in JSON form. */
const KeyName = schema.tuple([schema.string()], schema.json());

/** A change of one tracked row between instances. */
export const TrackerChange = schema.object({
    /** The table's SQL name. */
    table: schema.string(),
    /** The row's name: its table and key as JSON. */
    key: schema.string(),
    /** The row's values in JSON form, null once deleted. */
    row: schema.record(schema.string(), schema.json()).nullable(),
    /** The owner whose end removes the row. */
    owner: schema.string(),
});
/** A change of one tracked row between instances. */
export type TrackerChange = schema.Infer<typeof TrackerChange>;

/** A message between the instances tracking the same tables. */
export const TrackerMessage = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("write"),
        instance: schema.string(),
        rows: schema.array(TrackerChange),
    }),
    schema.object({
        kind: schema.literal("end"),
        instance: schema.string(),
        owner: schema.string(),
    }),
    schema.object({
        kind: schema.literal("track"),
        instance: schema.string(),
        owner: schema.string(),
    }),
    schema.object({
        kind: schema.literal("release"),
        instance: schema.string(),
        owner: schema.string(),
    }),
    schema.object({
        kind: schema.literal("broadcast"),
        instance: schema.string(),
        topic: schema.string(),
        event: schema.json(),
    }),
    schema.object({ kind: schema.literal("hello"), instance: schema.string() }),
    schema.object({
        kind: schema.literal("state"),
        instance: schema.string(),
        rows: schema.array(TrackerChange),
        tracked: schema.array(schema.string()),
    }),
    schema.object({ kind: schema.literal("heartbeat"), instance: schema.string() }),
    schema.object({ kind: schema.literal("stop"), instance: schema.string() }),
]);
/** A message between the instances tracking the same tables. */
export type TrackerMessage = schema.Infer<typeof TrackerMessage>;

/** How a tracker announces itself. */
export interface TrackerOptions {
    /** This instance's identifier. */
    readonly instance?: string;
    /** The heartbeat interval, in milliseconds. */
    readonly heartbeat?: number;
}

/** Rows of tables kept in memory on every instance and removed with their owners. */
export class Tracker {
    /** This instance's identifier. */
    readonly instance: string;
    /** The database of the tracked tables. */
    readonly database: DatabaseConnection;
    /** The tracked tables by SQL name. */
    readonly #tables: ReadonlyMap<string, Table>;
    /** The channel reaching the other instances. */
    readonly #channel: Channel<TrackerMessage>;
    /** Each owner's rows and their last writing instance. */
    readonly #owners = new Map<string, { instance: string; keys: Set<string> }>();
    /** The owner of each row, by row name. */
    readonly #rows = new Map<string, string>();
    /** The last heartbeat of each other instance, in UTC epoch milliseconds. */
    readonly #instances = new Map<string, number>();
    /** The instances tracking each owner. */
    readonly #tracked = new Map<string, Set<string>>();
    /** The listeners of owner tracking changes. */
    readonly #watchers = new Set<(owner: string, isTracked: boolean) => void>();
    /** The listeners of each broadcast topic. */
    readonly #listeners = new Map<string, Set<(event: JsonValue) => void>>();
    /** The heartbeat interval, in milliseconds. */
    readonly #heartbeat: number;
    /** The heartbeat timer. */
    readonly #beat: ReturnType<typeof setInterval>;
    /** Stop listening to the channel. */
    readonly #stop: () => void;
    /** The pending applications of other instances' messages. */
    #applied: Promise<void> = Promise.resolve();

    /** Track tables of a database on a channel. */
    constructor(
        database: DatabaseConnection,
        tables: readonly Table[],
        channel: Channel<unknown>,
        options: TrackerOptions = {},
    ) {
        // require every column in the log
        for (const table of tables) {
            const { columns, logged, name } = table[TABLE];
            const unlogged = Object.keys(columns).find(
                (property) => !Object.hasOwn(logged, property),
            );
            if (unlogged !== undefined) {
                throw new TypeError(`tracked table ${name} leaves ${unlogged} out of its log`);
            }
        }

        // keep the tables and announce this instance
        this.instance = options.instance ?? crypto.randomUUID();
        this.database = database;
        this.#tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
        this.#channel = typedChannel(channel, TrackerMessage);
        this.#heartbeat = options.heartbeat ?? HEARTBEAT_MILLISECONDS;

        // apply messages in order and ask for live rows on each start or resume
        this.#stop = this.#channel.listen(
            (message) => {
                this.#applied = this.#applied.then(() => this.#receive(message));
            },
            () => this.#channel.notify({ kind: "hello", instance: this.instance }),
        );

        // announce this instance and drop silent ones
        this.#beat = setInterval(() => {
            channel.notify({ kind: "heartbeat", instance: this.instance });
            const deadline = Date.now() - this.#heartbeat * MISSED_HEARTBEATS;
            for (const [instance, seen] of this.#instances) {
                if (seen < deadline) {
                    this.#applied = this.#applied.then(() => this.#drop(instance));
                }
            }
        }, this.#heartbeat);
    }

    /** Read the rows an open transaction wrote to the tracked tables. */
    async record(
        transaction: DatabaseConnection,
        ownerOf: (table: Table) => string,
    ): Promise<TrackerChange[]> {
        // keep each row's last image
        const images = new Map<string, TrackerChange>();
        for (const change of await transaction.log.written([...this.#tables.values()])) {
            const key = Key.name(change.table, change.key);
            const after = Change.after(change);
            const row = after === null ? null : change.table[TABLE].encode(after);
            images.set(key, {
                table: change.table[TABLE].sqlName,
                key,
                row,
                owner: ownerOf(change.table),
            });
        }

        return [...images.values()];
    }

    /** Publish committed rows to the other instances. */
    publish(rows: readonly TrackerChange[]): void {
        if (rows.length === 0) {
            return;
        }
        for (const row of rows) {
            this.#keep(row, this.instance);
        }
        this.#channel.notify({ kind: "write", instance: this.instance, rows: [...rows] });
    }

    /** End an owner everywhere. */
    async end(owner: string): Promise<void> {
        await this.#end(owner);
        this.#channel.notify({ kind: "end", instance: this.instance, owner });
    }

    /** Track an owner on this instance. */
    track(owner: string): void {
        this.#track(owner, this.instance);
        this.#channel.notify({ kind: "track", instance: this.instance, owner });
    }

    /** Release an owner this instance tracked. */
    release(owner: string): void {
        this.#release(owner, this.instance);
        this.#channel.notify({ kind: "release", instance: this.instance, owner });
    }

    /** Decide whether any instance tracks an owner alive. */
    isTracked(owner: string): boolean {
        return this.#tracked.has(owner);
    }

    /** Tell a listener when an owner becomes tracked or released, until stopped. */
    watchTracked(listener: (owner: string, isTracked: boolean) => void): () => void {
        this.#watchers.add(listener);

        return () => this.#watchers.delete(listener);
    }

    /** List the owners with their last writing instance. */
    owners(): Map<string, string> {
        return new Map([...this.#owners].map(([owner, entry]) => [owner, entry.instance]));
    }

    /** Send an event to a topic on every instance. */
    broadcast(topic: string, event: JsonValue): void {
        this.#deliver(topic, event);
        this.#channel.notify({ kind: "broadcast", instance: this.instance, topic, event });
    }

    /** Listen to a topic until stopped. */
    listen(topic: string, receive: (event: JsonValue) => void): () => void {
        const listeners = this.#listeners.get(topic) ?? new Set();
        this.#listeners.set(topic, listeners.add(receive));

        return () => {
            listeners.delete(receive);
            if (listeners.size === 0) {
                this.#listeners.delete(topic);
            }
        };
    }

    /** Wait until every received message is applied. */
    settled(): Promise<void> {
        return this.#applied;
    }

    /** Stop tracking and tell the others to drop this instance's rows. */
    close(): void {
        clearInterval(this.#beat);
        this.#channel.notify({ kind: "stop", instance: this.instance });
        this.#stop();
    }

    /** Apply another instance's message. */
    async #receive(message: TrackerMessage): Promise<void> {
        // note the sender as alive
        if (message.kind !== "stop") {
            this.#instances.set(message.instance, Date.now());
        }

        // apply the rows, ends and broadcasts
        if (message.kind === "write") {
            await this.#apply(message.rows, message.instance);
        } else if (message.kind === "state") {
            await this.#apply(message.rows, message.instance);
            for (const owner of message.tracked) {
                this.#track(owner, message.instance);
            }
        } else if (message.kind === "end") {
            await this.#end(message.owner);
        } else if (message.kind === "track") {
            this.#track(message.owner, message.instance);
        } else if (message.kind === "release") {
            this.#release(message.owner, message.instance);
        } else if (message.kind === "broadcast") {
            this.#deliver(message.topic, message.event);
        }
        // answer a hello with this instance's rows
        else if (message.kind === "hello") {
            const tracked = [...this.#tracked]
                .filter(([, instances]) => instances.has(this.instance))
                .map(([owner]) => owner);
            this.#channel.notify({
                kind: "state",
                instance: this.instance,
                rows: await this.#state(),
                tracked,
            });
        }
        // drop a stopped instance's rows
        else if (message.kind === "stop") {
            await this.#drop(message.instance);
        }
    }

    /** Write another instance's rows into this instance's tables. */
    async #apply(rows: readonly TrackerChange[], instance: string): Promise<void> {
        await this.database.transaction(async (transaction) => {
            for (const tracked of rows) {
                // upsert a live row and remove a deleted one
                const table = this.#table(tracked.table);
                if (tracked.row === null) {
                    await transaction
                        .delete(table)
                        .where(Key.match(table, Key.parse(table, tracked.key)));
                } else {
                    await transaction.upsert(table, [table[TABLE].decode(tracked.row)]);
                }
            }
        });
        for (const tracked of rows) {
            this.#keep(tracked, instance);
        }
    }

    /** Keep a row under its owner, or forget it once deleted. */
    #keep(tracked: TrackerChange, instance: string): void {
        // move the row from a previous owner
        const previous = this.#rows.get(tracked.key);
        if (previous !== undefined && previous !== tracked.owner) {
            this.#forget(previous, tracked.key);
        }

        // forget a deleted row or keep a live one
        if (tracked.row === null) {
            this.#forget(tracked.owner, tracked.key);
        } else {
            const entry = this.#owners.get(tracked.owner) ?? { instance, keys: new Set<string>() };
            entry.instance = instance;
            entry.keys.add(tracked.key);
            this.#owners.set(tracked.owner, entry);
            this.#rows.set(tracked.key, tracked.owner);
        }
    }

    /** Forget one row of an owner and the owner when it has none. */
    #forget(owner: string, key: string): void {
        // forget the row and an empty owner
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

        // delete them, a chain of keys per statement
        const byTable = Map.groupBy(entry.keys, (key) => tableOf(key));
        await this.database.transaction(async (transaction) => {
            for (const [name, keys] of byTable) {
                const table = this.#table(name);
                const matches = keys.map((key) => Key.match(table, Key.parse(table, key)));
                for (let start = 0; start < matches.length; start += CHAIN_TERMS) {
                    const [first, ...rest] = matches.slice(start, start + CHAIN_TERMS);
                    if (first !== undefined) {
                        await transaction.delete(table).where(or(first, ...rest));
                    }
                }
            }
        });
    }

    /** Remove every row and tracked owner of an instance. */
    async #drop(instance: string): Promise<void> {
        this.#instances.delete(instance);
        for (const owner of this.#tracked.keys()) {
            this.#release(owner, instance);
        }
        for (const [owner, entry] of this.#owners) {
            if (entry.instance === instance) {
                await this.#end(owner);
            }
        }
    }

    /** Read the rows this instance last wrote. */
    async #state(): Promise<TrackerChange[]> {
        const rows: TrackerChange[] = [];
        for (const [owner, entry] of this.#owners) {
            if (entry.instance !== this.instance) {
                continue;
            }
            for (const key of entry.keys) {
                const table = this.#table(tableOf(key));
                const [row] = await this.database
                    .select()
                    .from(table)
                    .where(Key.match(table, Key.parse(table, key)));
                if (row === undefined) {
                    throw new TypeError(`tracked row ${key} is missing from its table`);
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

    /** Note an instance tracking an owner. */
    #track(owner: string, instance: string): void {
        // tell the listeners on the first tracking instance
        const instances = this.#tracked.get(owner) ?? new Set<string>();
        const isFirst = instances.size === 0;
        this.#tracked.set(owner, instances.add(instance));
        if (isFirst) {
            for (const listener of this.#watchers) {
                listener(owner, true);
            }
        }
    }

    /** Note an instance releasing an owner. */
    #release(owner: string, instance: string): void {
        // tell the listeners on the last releasing instance
        const instances = this.#tracked.get(owner);
        if (instances === undefined || !instances.delete(instance) || instances.size > 0) {
            return;
        }
        this.#tracked.delete(owner);
        for (const listener of this.#watchers) {
            listener(owner, false);
        }
    }

    /** Read a tracked table by its SQL name, refusing an untracked one. */
    #table(name: string): Table {
        const table = this.#tables.get(name);
        if (table === undefined) {
            throw new TypeError(`table ${name} is not tracked`);
        }

        return table;
    }

    /** Hand an event to a topic's listeners on this instance. */
    #deliver(topic: string, event: JsonValue): void {
        for (const listener of this.#listeners.get(topic) ?? []) {
            listener(event);
        }
    }
}

/** Read the SQL name of the table a row's name starts with. */
function tableOf(key: string): string {
    const [table] = KeyName.parse(JSON.parse(key));

    return table;
}
