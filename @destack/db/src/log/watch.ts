import { type Channel, type Commit, HEARD, type Hearing } from "../channel/channel.ts";
import type { LogPosition } from "./position.ts";
import { LOG_TABLES } from "./schema.ts";

/** Who announces a connection's commits: the connection after commit, or the database inside it. */
export type Announcer = "connection" | "database";

/** The commits one physical connection's readers wait for. */
export class CommitWatch {
    /** The channel other writers announce their commits on, absent for a sole writer. */
    readonly #channel: Channel<Commit> | undefined;
    /** The SQL names of the tables whose commits the database announces inside the commit. */
    readonly #announced: ReadonlySet<string>;
    /** The SQL names of every declared table, which a commit of unknown tables lists. */
    readonly #tables: readonly string[];
    /** The SQL names of the tables without change triggers, which write no log entries. */
    readonly #unlogged: ReadonlySet<string>;
    /** The version each table's cached reads are valid at, by SQL name. */
    readonly #versions: Map<string, number>;
    /** The latest version given, counting every commit heard and every expiry. */
    #version = 0;
    /** Whether the watch hears other writers' commits: delivery started and has not failed. */
    #isHearing = false;
    /** The readers waiting for the next commit. */
    readonly #waiting = new Set<(failure?: unknown) => void>();
    /** The tables committed since each waiting reader's last check. */
    readonly #committed = new Set<Set<string>>();
    /** Stop listening for other writers' commits. */
    #stop?: () => Promise<void>;
    /** The failure that ended listening. */
    #failure?: { readonly error: unknown };
    /** Settle once the watch hears other writers' commits or stops, failing as listening fails. */
    readonly #listening: Promise<void>;
    /** Settle the listening wait. */
    readonly #settleListening: () => void;
    /** Fail the listening wait. */
    readonly #failListening: (error: unknown) => void;
    /** The latest sequence an announced commit ended at, all earlier ones committed, else zero. */
    #sequence = 0;
    /** The latest position whose written tables a read of the log woke, absent before the first. */
    #caughtUp: Omit<LogPosition, "log"> | undefined;
    /** The wakes since the watch began: each commit heard, delivery start and stop. */
    #generation = 0;
    /** The waits for this connection's commit tokens to reach every listening party. */
    readonly #hearings = new Map<string, Hearing>();
    /** The tokens of this connection's commits the channel has yet to deliver. */
    readonly #own = new Set<string>();

    /** Watch the commits announced on a channel, or only this connection's without one. */
    constructor(
        channel: Channel<Commit> | undefined,
        announced: ReadonlySet<string>,
        tables: readonly string[],
        unlogged: ReadonlySet<string>,
    ) {
        // store the channel, the tables the database announces, the declared and the unlogged tables
        this.#channel = channel;
        this.#announced = announced;
        this.#tables = tables;
        this.#unlogged = unlogged;

        // version every declared and log table from the watch's start
        this.#versions = new Map([...tables, ...LOG_TABLES].map((table) => [table, 0]));

        // hear a sole writer's commits at once, and other writers' once listening
        const { promise, resolve, reject } = Promise.withResolvers<void>();
        promise.catch(() => undefined);
        this.#listening = promise;
        this.#settleListening = resolve;
        this.#failListening = reject;
        if (channel === undefined) {
            resolve();
        }
    }

    /** The latest sequence an announced commit ended at, all earlier ones committed, else zero. */
    get sequence(): number {
        return this.#sequence;
    }

    /** The latest position whose written tables a read of the log woke, absent before the first. */
    get caughtUp(): Omit<LogPosition, "log"> | undefined {
        return this.#caughtUp;
    }

    /** The wakes since the watch began: each commit heard, delivery start and stop. */
    get generation(): number {
        return this.#generation;
    }

    /** Whether the watch listens for the commits announced on its channel. */
    get isListening(): boolean {
        return this.#stop !== undefined;
    }

    /** Whether the watch hears every commit: a sole writer's, or a channel's while delivered. */
    get isComplete(): boolean {
        return this.#channel === undefined || (this.#isHearing && this.#failure === undefined);
    }

    /** Read the generation of the latest commit heard writing each of some versioned tables. */
    versions(tables: readonly string[]): number[] {
        return tables.map((table) => {
            const version = this.#versions.get(table);
            if (version === undefined) {
                throw new TypeError(`table ${table} has no commit version`);
            }

            return version;
        });
    }

    /** Decide whether the watch versions a table: a declared table or one of the log's. */
    isVersioned(table: string): boolean {
        return this.#versions.has(table);
    }

    /** Wait until every party listening in this process heard a commit's token. */
    expect(token: string): Promise<void> {
        // remember a token heard while listening as this connection's, then wait for every party
        if (this.isListening) {
            this.#own.add(token);
        }
        const hearing = this.#channel?.hear(token) ?? HEARD;
        this.#hearings.set(token, hearing);

        return hearing.heard.finally(() => this.#hearings.delete(token));
    }

    /** Stop waiting for the token of a transaction that never commits. */
    forget(token: string): void {
        this.#own.delete(token);
        this.#hearings.get(token)?.forget();
        this.#hearings.delete(token);
    }

    /** Whether commits announce on a channel, which other connections listen to. */
    get isShared(): boolean {
        return this.#channel !== undefined;
    }

    /** Whether readers wait for a commit. */
    get isWaiting(): boolean {
        return this.#waiting.size > 0;
    }

    /** Wake a commit's readers here, and announce what the database leaves unannounced. */
    notify(tables: ReadonlySet<string>, scripted: ReadonlySet<string>): void {
        // wake this connection's readers of every written table, which it hears before the channel does
        this.#wake([...tables]);

        // announce the tables the database leaves unannounced, and every table a script changed
        const announced = [...tables].filter(
            (table) => !this.#announced.has(table) || scripted.has(table),
        );
        if (this.#channel !== undefined && announced.length > 0) {
            this.#channel.notify({ kind: "commit", tables: announced });
        }
    }

    /** Fail every waiting and later reader. */
    fail(error: unknown): void {
        // record the failure, end the listening wait and wake the readers
        this.#failure = { error };
        this.#isHearing = false;
        this.#failListening(error);
        const waiting = [...this.#waiting];
        this.#waiting.clear();
        for (const wake of waiting) {
            wake(error);
        }
    }

    /** Wait until a check of the tables committed since the last check passes after a commit. */
    async until(
        check: (tables: ReadonlySet<string>) => Promise<boolean>,
        signal: AbortSignal,
        generation?: number,
    ): Promise<boolean> {
        // collect each commit's tables from here on, starting from every table unless no wake came since the generation
        const committed = new Set(generation === this.#generation ? [] : this.#tables);
        this.#committed.add(committed);
        try {
            // check first once the watch hears other writers' commits
            await this.#listened(signal);
            while (!signal.aborted) {
                // refuse waiting on a channel that failed
                if (this.#failure) {
                    throw this.#failure.error;
                }

                // register before checking, and pass the tables committed since the last check
                const checked = new AbortController();
                const next = this.#next(AbortSignal.any([signal, checked.signal]));
                const tables: ReadonlySet<string> = new Set(committed);
                committed.clear();
                const isPassed = await check(tables).catch(async (error: unknown) => {
                    // settle the registered wait
                    checked.abort();
                    await Promise.allSettled([next]);
                    throw error;
                });
                if (isPassed) {
                    checked.abort();
                    await next;

                    return true;
                }
                await next;
            }
        } finally {
            this.#committed.delete(committed);
        }

        return false;
    }

    /** Listen for other writers' commits once, settling once they are heard. */
    listen(): Promise<void> {
        // listen once a reader asks, so a connection without readers never listens
        if (this.#channel !== undefined) {
            this.#stop ??= this.#listen();
        }

        return this.#listening;
    }

    /** Wake the readers of the tables commits up to a sequence wrote, as read from the log. */
    catchUp(position: Omit<LogPosition, "log">, tables: readonly string[] | "every"): void {
        // take the position, then wake the written tables and the log's, waking nobody without any
        this.#caughtUp = position;
        const written = tables === "every" ? this.#tables : tables;
        if (written.length > 0) {
            this.#advance(written, [...written, ...LOG_TABLES]);
        }
    }

    /** Wake every reader and stop listening for other writers. */
    async stop(): Promise<void> {
        // end the listening waits and wake every reader
        this.#settleListening();
        this.#isHearing = false;
        this.#wake("every");
        await this.#stop?.();
        await this.#channel?.close();
    }

    /** Wake the readers of tables here, versioning them, and the log's after a logged write. */
    #wake(tables: readonly string[] | "every"): void {
        const woken = tables === "every" ? this.#tables : tables;
        const isLogged = woken.some((table) => !this.#unlogged.has(table));
        this.#advance(woken, isLogged ? [...woken, ...LOG_TABLES] : woken);
    }

    /** Expire every cached read here, waking no reader, such as after a lost race. */
    expire(): void {
        this.#expire([...this.#tables, ...LOG_TABLES]);
    }

    /** Count a commit, version the known tables, and wake every reader with the committed ones. */
    #advance(committed: readonly string[], versioned: readonly string[]): void {
        // count the commit and version the tables the watch knows
        this.#generation += 1;
        this.#expire(versioned);

        // add the tables
        for (const tables of this.#committed) {
            for (const table of committed) {
                tables.add(table);
            }
        }

        // wake the readers
        const waiting = [...this.#waiting];
        this.#waiting.clear();
        for (const wake of waiting) {
            wake();
        }
    }

    /** Give the known tables a new version, which every read cached before misses. */
    #expire(tables: readonly string[]): void {
        this.#version += 1;
        for (const table of tables) {
            if (this.#versions.has(table)) {
                this.#versions.set(table, this.#version);
            }
        }
    }

    /** Wait until the watch listens or stops listening, or for the abort. */
    #listened(signal: AbortSignal): Promise<void> {
        return new Promise((resolve) => {
            // end at once for an aborted signal
            if (signal.aborted) {
                resolve();

                return;
            }

            // end at the abort, or once listening settles or fails, with no listener left
            const abort = () => resolve();
            const settle = () => {
                signal.removeEventListener("abort", abort);
                resolve();
            };
            signal.addEventListener("abort", abort, { once: true });
            this.listen().then(settle, settle);
        });
    }

    /** Wait for the next commit or the abort. */
    #next(signal: AbortSignal): Promise<void> {
        return new Promise((resolve, reject) => {
            // end at once when already aborted
            if (signal.aborted) {
                resolve();

                return;
            }

            // end at the next commit, failure or abort
            const wake = (failure?: unknown) => {
                signal.removeEventListener("abort", abort);
                if (failure === undefined) {
                    resolve();
                } else {
                    reject(failure);
                }
            };
            const abort = () => {
                this.#waiting.delete(wake);
                resolve();
            };
            this.#waiting.add(wake);
            signal.addEventListener("abort", abort, { once: true });
        });
    }

    /** Wake the readers on each announced commit, and on each resumed delivery for all tables. */
    #listen(): () => Promise<void> {
        const stop = this.#channel?.listen({
            receive: (message) => {
                // take the commit's sequence, and wake on another connection's commit
                this.#sequence = Math.max(this.#sequence, message.sequence ?? 0);
                const isOwn = message.token !== undefined && this.#own.delete(message.token);
                if (!isOwn) {
                    this.#wake(message.tables);
                }
            },
            resume: () => {
                // hear from here on, and wake every reader, as a reconnect may have lost notifications
                this.#isHearing = true;
                this.#own.clear();
                this.#settleListening();
                this.#wake("every");
            },
            fail: (error) => this.fail(error),
            pause: () => {
                // stop trusting what readers stored until delivery resumes
                this.#isHearing = false;
            },
        });

        return async () => stop?.();
    }
}
