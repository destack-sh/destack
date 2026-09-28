import {
    decodeRow,
    encodeRow,
    Key,
    or,
    TABLE,
    type DatabaseConnection,
    type JsonValue,
    type Row,
    type Table,
} from "@destack/db";
import { CHAIN_TERMS } from "@destack/db/query";
import type { Relay } from "@destack/db/relay";

/**
 * How often an instance announces that it is alive, in milliseconds.
 *
 * One small message per instance every 15 s costs nothing, and three missed beats bound a crashed instance's rows to 45 s.
 */
const HEARTBEAT_MILLISECONDS = 15_000;

/** How many heartbeats an instance may miss before the others drop its rows. */
const MISSED_HEARTBEATS = 3;

/** One row of a tracked table as it travels between instances: its key, its values or none once deleted, and its owner. */
export interface TrackedRow {
    /** The table's SQL name. */
    readonly table: string;
    /** The row's name: its table and key as JSON. */
    readonly key: string;
    /** The row's values in their JSON form, null once deleted. */
    readonly row: Readonly<Record<string, JsonValue>> | null;
    /** The owner whose end removes the row, such as a session or a call. */
    readonly owner: string;
}

/** A message between the instances tracking the same tables. */
export type TrackerMessage =
    | { readonly kind: "write"; readonly instance: string; readonly rows: readonly TrackedRow[] }
    | { readonly kind: "end"; readonly instance: string; readonly owner: string }
    | { readonly kind: "hold"; readonly instance: string; readonly owner: string }
    | { readonly kind: "release"; readonly instance: string; readonly owner: string }
    | {
          readonly kind: "broadcast";
          readonly instance: string;
          readonly topic: string;
          readonly event: JsonValue;
      }
    | { readonly kind: "hello"; readonly instance: string }
    | {
          readonly kind: "state";
          readonly instance: string;
          readonly rows: readonly TrackedRow[];
          readonly holds: readonly string[];
      }
    | { readonly kind: "heartbeat"; readonly instance: string }
    | { readonly kind: "stop"; readonly instance: string };

/** How a tracker announces itself. */
export interface TrackerOptions {
    /** This instance's identifier, unique among the instances sharing the relay. */
    readonly instance?: string;
    /** How often this instance announces that it is alive, in milliseconds. */
    readonly heartbeat?: number;
}

/**
 * Rows of tables held in memory on every instance, replicated between them and removed with their owners, never stored.
 *
 * Each instance writes rows into its own database and publishes them; the others apply them into theirs.
 * Rows go when their owner ends, or when the instance that last wrote them stops or falls silent.
 */
export class Tracker {
    /** This instance's identifier. */
    readonly instance: string;
    /** The database holding the tracked tables on this instance. */
    readonly database: DatabaseConnection;
    /** The tracked tables by SQL name. */
    readonly #tables: ReadonlyMap<string, Table>;
    /** The relay reaching the other instances. */
    readonly #relay: Relay<TrackerMessage>;
    /** Each owner's rows and the instance that last wrote them, by owner. */
    readonly #owners = new Map<string, { instance: string; keys: Set<string> }>();
    /** The owner of each row, by row name. */
    readonly #rows = new Map<string, string>();
    /** When each other instance last announced itself, in UTC epoch milliseconds. */
    readonly #instances = new Map<string, number>();
    /** The instances holding each owner alive, by owner. */
    readonly #holds = new Map<string, Set<string>>();
    /** The listeners told when an owner becomes held or released by every instance. */
    readonly #holders = new Set<(owner: string, isHeld: boolean) => void>();
    /** The listeners of each broadcast topic. */
    readonly #listeners = new Map<string, Set<(event: JsonValue) => void>>();
    /** How often this instance announces itself, in milliseconds. */
    readonly #heartbeat: number;
    /** The timer announcing this instance and dropping silent ones. */
    readonly #beat: ReturnType<typeof setInterval>;
    /** Stop listening to the relay. */
    readonly #stop: () => void;
    /** The applications of other instances' messages, in arrival order. */
    #applied: Promise<void> = Promise.resolve();

    /** Track tables of a database with the other instances listening on a relay. */
    constructor(
        database: DatabaseConnection,
        tables: readonly Table[],
        relay: Relay<TrackerMessage>,
        options: TrackerOptions = {},
    ) {
        // require every column in the log, since rows travel as the log records them
        for (const table of tables) {
            const { columns, logged, name } = table[TABLE];
            const unlogged = Object.keys(columns).find(
                (property) => !Object.hasOwn(logged, property),
            );
            if (unlogged !== undefined) {
                throw new TypeError(`tracked table ${name} leaves ${unlogged} out of its log`);
            }
        }

        // hold the tables and announce this instance
        this.instance = options.instance ?? crypto.randomUUID();
        this.database = database;
        this.#tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
        this.#relay = relay;
        this.#heartbeat = options.heartbeat ?? HEARTBEAT_MILLISECONDS;

        // apply the others' messages in order, asking for every live row whenever delivery starts or resumes
        this.#stop = relay.listen(
            (message) => {
                this.#applied = this.#applied.then(() => this.#receive(message));
            },
            () => relay.post({ kind: "hello", instance: this.instance }),
        );

        // announce this instance, and drop the rows of instances that went silent
        this.#beat = setInterval(() => {
            relay.post({ kind: "heartbeat", instance: this.instance });
            const deadline = Date.now() - this.#heartbeat * MISSED_HEARTBEATS;
            for (const [instance, seen] of this.#instances) {
                if (seen < deadline) {
                    this.#applied = this.#applied.then(() => this.#drop(instance));
                }
            }
        }, this.#heartbeat);
    }

    /** Read what an open transaction wrote to the tracked tables, each row owned by its table's owner, to publish once it commits. */
    async record(
        transaction: DatabaseConnection,
        ownerOf: (table: Table) => string,
    ): Promise<TrackedRow[]> {
        // keep the last image of each row the transaction wrote
        const images = new Map<string, TrackedRow>();
        for (const change of await transaction.log.written([...this.#tables.values()])) {
            const key = Key.name(change.table, change.key);
            const row =
                change.after === undefined ? null : encodeRow(change.table, change.after as Row);
            images.set(key, {
                table: change.table[TABLE].sqlName,
                key,
                row,
                owner: ownerOf(change.table),
            });
        }

        return [...images.values()];
    }

    /** Publish rows a committed transaction wrote to the other instances, holding their owners here. */
    publish(rows: readonly TrackedRow[]): void {
        if (rows.length === 0) {
            return;
        }
        for (const row of rows) {
            this.#own(row, this.instance);
        }
        this.#relay.post({ kind: "write", instance: this.instance, rows });
    }

    /** End an owner: remove its rows here and on every other instance. */
    async end(owner: string): Promise<void> {
        await this.#end(owner);
        this.#relay.post({ kind: "end", instance: this.instance, owner });
    }

    /** Hold an owner alive on this instance, such as while a session's stream is open here. */
    hold(owner: string): void {
        this.#hold(owner, this.instance);
        this.#relay.post({ kind: "hold", instance: this.instance, owner });
    }

    /** Release an owner this instance held. */
    release(owner: string): void {
        this.#release(owner, this.instance);
        this.#relay.post({ kind: "release", instance: this.instance, owner });
    }

    /** Decide whether any instance holds an owner alive. */
    isHeld(owner: string): boolean {
        return this.#holds.has(owner);
    }

    /** Tell a listener whenever an owner becomes held by some instance or released by every one, until the returned stop runs. */
    watchHolds(listener: (owner: string, isHeld: boolean) => void): () => void {
        this.#holders.add(listener);

        return () => this.#holders.delete(listener);
    }

    /** List the owners holding rows, with the instance that last wrote each. */
    owners(): Map<string, string> {
        return new Map([...this.#owners].map(([owner, held]) => [owner, held.instance]));
    }

    /** Send an event to the listeners of a topic on every instance, never stored. */
    broadcast(topic: string, event: JsonValue): void {
        this.#deliver(topic, event);
        this.#relay.post({ kind: "broadcast", instance: this.instance, topic, event });
    }

    /** Listen to a topic's events until the returned stop runs. */
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

    /** Wait until every message received so far is applied. */
    settled(): Promise<void> {
        return this.#applied;
    }

    /** Stop tracking, telling the others to drop the rows this instance last wrote. */
    close(): void {
        clearInterval(this.#beat);
        this.#relay.post({ kind: "stop", instance: this.instance });
        this.#stop();
    }

    /** Apply another instance's message. */
    async #receive(message: TrackerMessage): Promise<void> {
        // note the sender as alive
        if (message.kind !== "stop") {
            this.#instances.set(message.instance, Date.now());
        }

        // apply its rows, their owners' ends and its broadcasts
        if (message.kind === "write") {
            await this.#apply(message.rows, message.instance);
        } else if (message.kind === "state") {
            await this.#apply(message.rows, message.instance);
            for (const owner of message.holds) {
                this.#hold(owner, message.instance);
            }
        } else if (message.kind === "end") {
            await this.#end(message.owner);
        } else if (message.kind === "hold") {
            this.#hold(message.owner, message.instance);
        } else if (message.kind === "release") {
            this.#release(message.owner, message.instance);
        } else if (message.kind === "broadcast") {
            this.#deliver(message.topic, message.event);
        }
        // answer a new or resumed instance with the rows this instance last wrote
        else if (message.kind === "hello") {
            const holds = [...this.#holds]
                .filter(([, instances]) => instances.has(this.instance))
                .map(([owner]) => owner);
            this.#relay.post({
                kind: "state",
                instance: this.instance,
                rows: await this.#state(),
                holds,
            });
        }
        // drop a stopped instance's rows
        else if (message.kind === "stop") {
            await this.#drop(message.instance);
        }
    }

    /** Write rows another instance published into this instance's tables. */
    async #apply(rows: readonly TrackedRow[], instance: string): Promise<void> {
        await this.database.transaction(async (transaction) => {
            for (const tracked of rows) {
                // upsert a live row, and remove a deleted one
                const table = this.#tables.get(tracked.table)!;
                if (tracked.row === null) {
                    await transaction
                        .delete(table)
                        .where(Key.match(table, Key.parse(table, tracked.key)));
                } else {
                    await transaction.upsert(table, [decodeRow(table, tracked.row)]);
                }
            }
        });
        for (const tracked of rows) {
            this.#own(tracked, instance);
        }
    }

    /** Hold a row under its owner, or forget it once deleted. */
    #own(tracked: TrackedRow, instance: string): void {
        // move the row from a previous owner
        const previous = this.#rows.get(tracked.key);
        if (previous !== undefined && previous !== tracked.owner) {
            this.#forget(previous, tracked.key);
        }

        // forget a deleted row, and hold a live one under the instance that last wrote its owner's rows
        if (tracked.row === null) {
            this.#forget(tracked.owner, tracked.key);
        } else {
            const held = this.#owners.get(tracked.owner) ?? { instance, keys: new Set<string>() };
            held.instance = instance;
            held.keys.add(tracked.key);
            this.#owners.set(tracked.owner, held);
            this.#rows.set(tracked.key, tracked.owner);
        }
    }

    /** Forget one row of an owner, and the owner once it holds none. */
    #forget(owner: string, key: string): void {
        // forget the row, and the owner once empty
        const held = this.#owners.get(owner);
        held?.keys.delete(key);
        this.#rows.delete(key);
        if (held?.keys.size === 0) {
            this.#owners.delete(owner);
        }
    }

    /** Remove an owner's rows from this instance's tables. */
    async #end(owner: string): Promise<void> {
        // forget the owner's rows
        const held = this.#owners.get(owner);
        if (held === undefined) {
            return;
        }
        this.#owners.delete(owner);
        for (const key of held.keys) {
            this.#rows.delete(key);
        }

        // delete them, a chain of keys per statement for each table
        const byTable = Map.groupBy(held.keys, (key) => JSON.parse(key)[0] as string);
        await this.database.transaction(async (transaction) => {
            for (const [name, keys] of byTable) {
                const table = this.#tables.get(name)!;
                const matches = keys.map((key) => Key.match(table, Key.parse(table, key)));
                for (let start = 0; start < matches.length; start += CHAIN_TERMS) {
                    await transaction
                        .delete(table)
                        .where(or(...matches.slice(start, start + CHAIN_TERMS))!);
                }
            }
        });
    }

    /** Remove every row whose owner another instance last wrote for, and every hold it kept. */
    async #drop(instance: string): Promise<void> {
        this.#instances.delete(instance);
        for (const owner of this.#holds.keys()) {
            this.#release(owner, instance);
        }
        for (const [owner, held] of this.#owners) {
            if (held.instance === instance) {
                await this.#end(owner);
            }
        }
    }

    /** Read the rows this instance last wrote, as a new instance receives them. */
    async #state(): Promise<TrackedRow[]> {
        const rows: TrackedRow[] = [];
        for (const [owner, held] of this.#owners) {
            if (held.instance !== this.instance) {
                continue;
            }
            for (const key of held.keys) {
                const table = this.#tables.get(JSON.parse(key)[0] as string)!;
                const [row] = await this.database
                    .select()
                    .from(table)
                    .where(Key.match(table, Key.parse(table, key)));
                rows.push({
                    table: table[TABLE].sqlName,
                    key,
                    row: encodeRow(table, row as Row),
                    owner,
                });
            }
        }

        return rows;
    }

    /** Note an instance holding an owner, telling the listeners once the first one does. */
    #hold(owner: string, instance: string): void {
        // note the instance, telling the listeners when it is the first
        const instances = this.#holds.get(owner) ?? new Set<string>();
        const isFirst = instances.size === 0;
        this.#holds.set(owner, instances.add(instance));
        if (isFirst) {
            for (const listener of this.#holders) {
                listener(owner, true);
            }
        }
    }

    /** Note an instance releasing an owner, telling the listeners once no instance holds it. */
    #release(owner: string, instance: string): void {
        // forget the instance, telling the listeners when no other is left
        const instances = this.#holds.get(owner);
        if (instances === undefined || !instances.delete(instance) || instances.size > 0) {
            return;
        }
        this.#holds.delete(owner);
        for (const listener of this.#holders) {
            listener(owner, false);
        }
    }

    /** Hand an event to a topic's listeners on this instance. */
    #deliver(topic: string, event: JsonValue): void {
        for (const listener of this.#listeners.get(topic) ?? []) {
            listener(event);
        }
    }
}
