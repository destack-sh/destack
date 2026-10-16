import {
    Key,
    TABLE,
    type DatabaseConnection,
    type Row,
    type Table,
    type Scalar,
    type LogPosition,
} from "@destack/db";
import type { Page } from "../../query/page.ts";
import type { Query } from "../../query/query.ts";
import type { Replica } from "../../replica/replica.ts";
import type { Audience } from "../audience.ts";
import type { Feed } from "../feed.ts";
import { groupName, type Contents } from "./oracle.ts";
import { Clock } from "@destack/environment";

/** What a subscriber has after applying its pages. */
export class MemoryReplica implements Contents {
    /** The tables the replica's rows belong to. */
    readonly #tables: ReadonlyMap<string, Table>;
    /** The rows by table and key, concealed columns missing. */
    readonly rows: Map<string, Record<string, unknown>>;
    /** The aggregate groups by query and group. */
    readonly results: Map<string, Record<string, Scalar>>;
    /** The pages applied, in order. */
    readonly pages: Page[];
    /** The position of the last complete page. */
    position: LogPosition | undefined;
    /** The waiters for the next page. */
    readonly #waiting: Set<() => void>;

    /** Keep rows of some tables. */
    constructor(tables: readonly Table[]) {
        // keep the tables, with no rows, groups or pages yet
        this.#tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
        this.rows = new Map();
        this.results = new Map();
        this.pages = [];
        this.position = undefined;
        this.#waiting = new Set();
    }

    /** Apply one page. */
    apply(page: Page): void {
        // record the page and reset on a snapshot
        this.pages.push(page);
        if (page.reset) {
            this.rows.clear();
            this.results.clear();
        }

        // keep and let go of rows and refuse contradicting changes
        for (const change of page.changes) {
            const table = this.#tables.get(change.table);
            if (table === undefined) {
                throw new TypeError(`page changes an unknown table: ${change.table}`);
            }
            const key = Key.name(table, MemoryReplica.#key(table, change.row));
            if ((change.operation === "insert") === this.rows.has(key)) {
                throw new Error(
                    `page ${change.operation}s a row the replica ${this.rows.has(key) ? "has" : "lacks"}: ${key}`,
                );
            }
            if (change.operation === "delete") {
                this.rows.delete(key);
            } else {
                const concealed = (change.concealed ?? []).map((name): [string, null] => [
                    name,
                    null,
                ]);
                this.rows.set(key, { ...change.row, ...Object.fromEntries(concealed) });
            }
        }

        // keep and let go of groups
        for (const result of page.results ?? []) {
            if (result.group === null) {
                for (const key of this.results.keys()) {
                    if (key.startsWith(JSON.stringify(result.query))) {
                        this.results.delete(key);
                    }
                }
            } else if (result.values === null) {
                if (!this.results.has(groupName(result.query, result.group))) {
                    throw new Error(
                        `page removes a group the replica lacks: ${groupName(result.query, result.group)}`,
                    );
                }
                this.results.delete(groupName(result.query, result.group));
            } else {
                this.results.set(groupName(result.query, result.group), result.values);
            }
        }

        // note the complete position and wake the waiters
        if (page.complete) {
            this.position = page.position;
        }
        for (const wake of this.#waiting) {
            wake();
        }
        this.#waiting.clear();
    }

    /** Wait for the next page. */
    next(): Promise<void> {
        return new Promise((resolve) => {
            this.#waiting.add(resolve);
        });
    }

    /** Read a row's key from its JSON form. */
    static #key(table: Table, row: Readonly<Record<string, unknown>>): Row {
        return Object.fromEntries(
            table[TABLE].key.map((property) => [
                property,
                table[TABLE].column(property).definition.fromJson(row[property]),
            ]),
        );
    }
}

/** A subscriber following a feed's queries into a replica. */
export class Follower {
    /** The feed followed. */
    readonly #feed: Feed;
    /** The subscriber's audience. */
    readonly #audience: Audience | undefined;
    /** The persisted replica and its database. */
    readonly #persisted:
        | { readonly replica: Replica; readonly database: DatabaseConnection }
        | undefined;
    /** What the subscriber has. */
    readonly replica: MemoryReplica;
    /** The queries followed. */
    #queries: Readonly<Record<string, Query>>;
    /** Stop the current stream. */
    #stop: AbortController;
    /** The current stream's loop. */
    #running: Promise<void>;
    /** The failure that stopped a stream. */
    #failure: Error | undefined;

    /** Create the follower. */
    constructor(
        feed: Feed,
        queries: Readonly<Record<string, Query>>,
        tables: readonly Table[],
        options: {
            readonly audience?: Audience;
            readonly replica?: { readonly replica: Replica; readonly database: DatabaseConnection };
        } = {},
    ) {
        // keep the feed, queries and tables
        this.#feed = feed;
        this.#queries = queries;
        this.#audience = options.audience;
        this.#persisted = options.replica;
        this.replica = new MemoryReplica(tables);
        this.#stop = new AbortController();
        this.#running = Promise.resolve();
    }

    /** Follow from the replica's position. */
    start(previous?: Readonly<Record<string, Query>>): void {
        // subscribe from the replica's position
        this.#stop = new AbortController();
        const signal = this.#stop.signal;
        const pages = this.#feed.subscribe(this.#queries, this.replica.position, signal, {
            ...(this.#audience === undefined ? {} : { audience: this.#audience }),
            ...(previous === undefined ? {} : { previous }),
        });
        const applied =
            this.#persisted === undefined
                ? pages
                : this.#persisted.replica.apply(this.#persisted.database, pages, {
                      clock: Clock.system,
                  });
        this.#running = (async () => {
            try {
                for await (const page of applied) {
                    this.replica.apply(page);
                }
            } catch (error) {
                if (!signal.aborted) {
                    this.#failure =
                        error instanceof Error
                            ? error
                            : new TypeError("following failed", { cause: error });
                }
            }
        })();
    }

    /** Reconnect from the replica's position, to other queries when given them. */
    async reconnect(queries?: Readonly<Record<string, Query>>): Promise<void> {
        // stop and follow the new queries
        await this.stop();
        const previous = queries === undefined ? undefined : this.#queries;
        this.#queries = queries ?? this.#queries;
        this.start(previous);
    }

    /** Stop following. */
    async stop(): Promise<void> {
        this.#stop.abort();
        await this.#running;
    }

    /** Wait until the replica reaches a position. */
    async wait(position: LogPosition): Promise<void> {
        for (;;) {
            if (this.#failure !== undefined) {
                throw this.#failure;
            }
            const reached = this.replica.position;
            if (
                reached !== undefined &&
                reached.epoch === position.epoch &&
                reached.sequence >= position.sequence
            ) {
                return;
            }
            await Promise.race([this.replica.next(), this.#running]);
        }
    }
}
