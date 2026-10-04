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

/** What a subscriber has after applying its pages. */
export class Copy implements Contents {
    /** The tables the copy's rows belong to. */
    readonly #tables: ReadonlyMap<string, Table>;
    /** The rows by table and key, concealed columns missing. */
    readonly rows = new Map<string, Record<string, unknown>>();
    /** The aggregate groups by query and group. */
    readonly results = new Map<string, Record<string, Scalar>>();
    /** The pages applied, in order. */
    readonly pages: Page[] = [];
    /** The position of the last complete page. */
    position: LogPosition | undefined;
    /** The waiters for the next page. */
    readonly #waiting = new Set<() => void>();

    /** Keep rows of some tables. */
    constructor(tables: readonly Table[]) {
        this.#tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
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
            const key = Key.name(table, keyOf(table, change.row));
            if ((change.operation === "insert") === this.rows.has(key)) {
                throw new Error(
                    `page ${change.operation}s a row the copy ${this.rows.has(key) ? "has" : "lacks"}: ${key}`,
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
                        `page removes a group the copy lacks: ${groupName(result.query, result.group)}`,
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
}

/** A subscriber following a feed's queries into a copy. */
export class Follower {
    /** The feed followed. */
    readonly #feed: Feed;
    /** The subscriber's audience. */
    readonly #audience: Audience | undefined;
    /** The replica and its database. */
    readonly #replica:
        | { readonly replica: Replica; readonly database: DatabaseConnection }
        | undefined;
    /** What the subscriber has. */
    readonly copy: Copy;
    /** The queries followed. */
    #queries: Readonly<Record<string, Query>>;
    /** Stop the current stream. */
    #stop = new AbortController();
    /** The current stream's loop. */
    #running: Promise<void> = Promise.resolve();
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
        this.#replica = options.replica;
        this.copy = new Copy(tables);
    }

    /** Follow from the copy's position. */
    start(previous?: Readonly<Record<string, Query>>): void {
        // subscribe from the copy's position
        this.#stop = new AbortController();
        const signal = this.#stop.signal;
        const pages = this.#feed.subscribe(this.#queries, this.copy.position, signal, {
            ...(this.#audience === undefined ? {} : { audience: this.#audience }),
            ...(previous === undefined ? {} : { previous }),
        });
        const applied =
            this.#replica === undefined
                ? pages
                : this.#replica.replica.apply(this.#replica.database, pages);
        this.#running = (async () => {
            try {
                for await (const page of applied) {
                    this.copy.apply(page);
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

    /** Reconnect from the copy's position, to other queries when given them. */
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

    /** Wait until the copy reaches a position. */
    async reach(position: LogPosition): Promise<void> {
        for (;;) {
            if (this.#failure !== undefined) {
                throw this.#failure;
            }
            const reached = this.copy.position;
            if (
                reached !== undefined &&
                reached.epoch === position.epoch &&
                reached.sequence >= position.sequence
            ) {
                return;
            }
            await Promise.race([this.copy.next(), this.#running]);
        }
    }
}

/** Read a row's key from its JSON form. */
function keyOf(table: Table, row: Readonly<Record<string, unknown>>): Row {
    return Object.fromEntries(
        table[TABLE].key.map((property) => [
            property,
            table[TABLE].column(property).definition.fromJson(row[property]),
        ]),
    );
}
