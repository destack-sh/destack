import type { Channel } from "@destack/db";
import type { Duration } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { broadcastChannel, connectShared } from "@destack/db/browser";
import type { ClientOptions } from "../client/package.ts";
import { RequestId } from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import { ObjectClient } from "../client/client.ts";
import type { Clock } from "@destack/environment";

/** The browser capabilities tabs share. */
export interface BrowserHost {
    /** The channel every tab and the database worker share. */
    readonly channel: Channel<unknown>;
    /** Take a named lock while a callback runs. */
    readonly request: (
        name: string,
        callback: () => Promise<void>,
        signal?: AbortSignal,
    ) => Promise<void>;
    /** Start the worker with the database, returning how to stop it. */
    readonly start: () => Promise<() => Promise<void>>;
}

/** One tab's objects over the database of the leading tab's worker. */
export class BrowserTab<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> {
    /** The tab's client. */
    readonly client: ObjectClient<Objects>;
    /** Settles once the database answers with its tables. */
    readonly ready: Promise<void>;
    /** The connection to the leading tab's worker. */
    readonly #database: ReturnType<typeof connectShared>;
    /** The browser the tab runs in. */
    readonly #host: BrowserHost;
    /** The database's name, shared by every tab of the origin. */
    readonly #name: string;
    /** Aborts the tab's loops and waits on close. */
    readonly #stopping: AbortController;
    /** Release the tab's presence lock. */
    readonly #leave: () => void;
    /** Settles once the tab no longer leads the database or waits to. */
    readonly #owning: Promise<void>;

    /** Join the tabs as one party. */
    private constructor(
        client: ObjectClient<Objects>,
        database: ReturnType<typeof connectShared>,
        ready: Promise<void>,
        host: BrowserHost,
        name: string,
        stopping: AbortController,
        leading: {
            readonly objects: Readonly<Record<string, ObjectType>>;
            readonly report: (error: unknown) => void;
        },
    ) {
        // take the presence lock
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

        // wait to lead the database
        const signal = this.#stopping.signal;
        const owner = `destack:${name}:owner`;
        this.#owning = host
            .request(owner, () => this.#lead(leading.objects, leading.report), signal)
            .catch((error: unknown) => {
                // rethrow unless closing
                if (!signal.aborted) {
                    throw error;
                }
            });
    }

    /** Open a tab's objects over the database of the leading tab's worker. */
    static async open<const Objects extends Readonly<Record<string, ObjectType>>>(options: {
        /** The database's name, shared by every tab of the origin. */
        readonly name: string;
        /** The object types to replicate, by key. */
        readonly objects: Objects;
        /** The scope whose objects to replicate. */
        readonly scope: string;
        /** The calling principal. */
        readonly caller: Subject;
        /** The browser's device, whose partition stores local objects scoped to devices. */
        readonly device: Subject;
        /** Where and how to call the server serving the objects. */
        readonly endpoint: ClientOptions;
        /** Where and how to call the machine serving a moved scope now. */
        readonly reconnect: (machine: string) => ClientOptions;
        /** Where and how to call the home of an enclosing scope whose rows the tab changes. */
        readonly homes?: (scope: string) => ClientOptions;
        /** The most object rows the shared replica stores, absent for no limit. */
        readonly storage?: { readonly rows: number };
        /** How long the shared local log retains changes, a minute by default. */
        readonly log?: { readonly retention: Duration };
        /** The browser capabilities, the page's by default. */
        readonly host?: BrowserHost;
        /** Report the leading tab's loop failures. */
        readonly report: (error: unknown) => void;
        /** The clock the tab's client reads time at. */
        readonly clock: Clock;
    }): Promise<BrowserTab<Objects>> {
        // connect to the shared database
        const host = options.host ?? BrowserTab.host(options.name);
        const origin = RequestId.create(options.clock.now());
        const { objects } = options;
        const database = connectShared(host.channel, origin, ObjectClient.tables(objects));
        const client = await ObjectClient.open({ ...options, database, origin, isMigrated: true });

        // settle once the tables exist
        const stopping = new AbortController();
        const ready = (async () => {
            await database.log.until(
                async () => (await ObjectClient.unapplied(database, objects)).length === 0,
                stopping.signal,
            );
        })();

        return new BrowserTab(client, database, ready, host, options.name, stopping, {
            objects,
            report: options.report,
        });
    }

    /** Bind the page's Web Locks, broadcast channel and database worker. */
    static host(name: string): BrowserHost {
        return {
            channel: broadcastChannel(`destack:${name}`),
            request: (lock, callback, signal) =>
                navigator.locks.request(lock, signal === undefined ? {} : { signal }, callback),
            start: async () => {
                // ask to persist the storage, and start the database worker
                await navigator.storage.persist();
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

    /** Lead the database until the tab closes, migrating its tables from this tab's build. */
    async #lead(
        objects: Readonly<Record<string, ObjectType>>,
        report: (error: unknown) => void,
    ): Promise<void> {
        // skip a closed tab
        const signal = this.#stopping.signal;
        if (signal.aborted) {
            return;
        }

        // start the worker and run the loops
        const stop = await this.#host.start();
        try {
            await ObjectClient.migrate(this.client.database, objects);
            await Promise.all([
                this.client.run(signal, report),
                this.#forgetAbsent(signal, report),
            ]);
        } finally {
            await stop();
        }
    }

    /** Forget each party with a closed tab. */
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

    /** Build the name of the lock a party's tab takes while it is open. */
    #presence(origin: string): string {
        return `destack:${this.#name}:tab:${origin}`;
    }
}
