import type { Subject } from "@destack/access";
import { broadcastRelay, type Relay } from "@destack/db/relay";
import { connectShared, type Message } from "@destack/db/shared";
import type { Table } from "@destack/db";
import type { Client } from "@destack/service";
import { RequestId } from "@destack/service/request";
import type { ReplicaProcedures } from "../replica/replica.ts";
import type { ObjectType } from "../object/object.ts";
import { ObjectClient } from "./client.ts";
import type { Duration } from "../object/duration.ts";

/** The browser capabilities tabs share. */
export interface BrowserHost {
    /** The relay every tab and the database worker share. */
    readonly relay: Relay<Message>;
    /** Hold a named lock while a callback runs. */
    request(name: string, hold: () => Promise<void>, signal?: AbortSignal): Promise<void>;
    /** Start the worker holding the database, returning how to stop it. */
    start(): Promise<() => Promise<void>>;
}

/** One tab's objects over the database the owning tab's worker holds. */
export class BrowserTab {
    /** The tab's client. */
    readonly client: ObjectClient;
    /** Settles once the database answers with its tables. */
    readonly ready: Promise<void>;
    /** The connection to the owning tab's worker. */
    readonly #database: ReturnType<typeof connectShared>;
    /** The browser the tab runs in. */
    readonly #host: BrowserHost;
    /** The database's name, shared by every tab of the origin. */
    readonly #name: string;
    /** Aborts the tab's loops and waits on close. */
    readonly #stopping: AbortController;
    /** Release the tab's presence lock. */
    readonly #leave: () => void;
    /** Settles once the tab no longer owns the database or waits to. */
    readonly #owning: Promise<void>;

    /** Join the tabs as one party. */
    private constructor(
        client: ObjectClient,
        database: ReturnType<typeof connectShared>,
        ready: Promise<void>,
        host: BrowserHost,
        name: string,
        stopping: AbortController,
        own: { readonly tables: readonly Table[]; readonly report: (error: unknown) => void },
    ) {
        // hold the presence lock
        this.client = client;
        this.ready = ready;
        this.#database = database;
        this.#host = host;
        this.#name = name;
        this.#stopping = stopping;
        let leave!: () => void;
        const left = new Promise<void>((resolve) => {
            leave = resolve;
        });
        this.#leave = leave;
        void host.request(this.#presence(client.origin), () => left);

        // wait to own the database
        const signal = this.#stopping.signal;
        const owner = `destack:${name}:owner`;
        this.#owning = host
            .request(owner, () => this.#own(own.tables, own.report), signal)
            .catch((error: unknown) => {
                // rethrow unless closing
                if (!signal.aborted) {
                    throw error;
                }
            });
    }

    /** Open a tab's objects over the database the owning tab's worker holds. */
    static async open<Object extends ObjectType>(options: {
        /** The database's name, shared by every tab of the origin. */
        readonly name: string;
        /** The object types to hold. */
        readonly objects: readonly Object[];
        /** The scope whose objects to hold. */
        readonly scope: string;
        /** The calling principal. */
        readonly caller: Subject;
        /** The service's replica procedures. */
        readonly service: Client<ReplicaProcedures>;
        /** Connect to the replica procedures of the holder a moved scope now answers at. */
        readonly reconnect: (holder: string) => Client<ReplicaProcedures>;
        /** The most object rows the shared copy holds, absent for no limit. */
        readonly storage?: { readonly rows: number };
        /** How long the shared local log keeps changes, a minute by default. */
        readonly log?: { readonly keep: Duration };
        /** The browser capabilities, the page's own by default. */
        readonly host?: BrowserHost;
        /** Report the owning tab's loop failures. */
        readonly report: (error: unknown) => void;
    }): Promise<BrowserTab> {
        // connect to the shared database
        const host = options.host ?? BrowserTab.host(options.name);
        const origin = RequestId.create();
        const tables = ObjectClient.tables(options.objects);
        const database = connectShared(host.relay, origin, tables);
        const client = await ObjectClient.open({ ...options, database, origin, isMigrated: true });

        // settle once the tables exist
        const stopping = new AbortController();
        const ready = (async () => {
            await database.log.until(
                async () => (await database.unapplied(tables, { isReplica: true })).length === 0,
                stopping.signal,
            );
        })();

        return new BrowserTab(client, database, ready, host, options.name, stopping, {
            tables,
            report: options.report,
        });
    }

    /** Bind the page's Web Locks, broadcast relay and database worker. */
    static host(name: string): BrowserHost {
        return {
            relay: broadcastRelay<Message>(`destack:${name}`),
            request: (lock, hold, signal) =>
                navigator.locks.request(lock, signal === undefined ? {} : { signal }, hold),
            start: async () => {
                // start the database worker
                const worker = new Worker(new URL("./database.worker.ts", import.meta.url), {
                    type: "module",
                    name: `destack:${name}`,
                });
                worker.postMessage({ name });

                return async () => worker.terminate();
            },
        };
    }

    /** Close the tab and hand over ownership. */
    async close(): Promise<void> {
        // stop, leave and wait for ownership to pass
        this.#stopping.abort();
        await this.client.close();
        this.#leave();
        await this.#owning;
        await this.#database.close();
    }

    /** Own the database until the tab closes. */
    async #own(tables: readonly Table[], report: (error: unknown) => void): Promise<void> {
        // skip a closed tab
        const signal = this.#stopping.signal;
        if (signal.aborted) {
            return;
        }

        // start the worker and run the loops
        const stop = await this.#host.start();
        try {
            await this.client.database.migrate(tables, { isReplica: true });
            await Promise.all([
                this.client.run(signal, report),
                this.#forgetAbsent(signal, report),
            ]);
        } finally {
            await stop();
        }
    }

    /** Forget each party whose tab closes. */
    async #forgetAbsent(signal: AbortSignal, report: (error: unknown) => void): Promise<void> {
        const watched = new Set<string>();
        const unwatched = async () =>
            [...(await this.client.origins())].filter(
                (origin) => origin !== this.client.origin && !watched.has(origin),
            );
        while (!signal.aborted) {
            // forget each new party once its presence lock frees
            for (const origin of await unwatched()) {
                watched.add(origin);
                const forget = () => this.client.forget(origin);
                void this.#host.request(this.#presence(origin), forget, signal).then(
                    () => watched.delete(origin),
                    (error: unknown) => {
                        // report unless closing
                        if (!signal.aborted) {
                            report(error);
                        }
                    },
                );
            }

            // wait for a party to appear
            await this.client.database.log.until(
                async () => (await unwatched()).length > 0,
                signal,
            );
        }
    }

    /** Name the lock a party's tab holds while it is open. */
    #presence(origin: string): string {
        return `destack:${this.#name}:tab:${origin}`;
    }
}
