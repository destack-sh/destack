import type { Channel, Commit } from "../channel/channel.ts";
import type { LogPosition } from "./position.ts";

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
    /** The commits heard since the watch began. */
    #generation = 0;
    /** The log head read since the latest commit heard, at that commit's generation. */
    #head: { readonly generation: number; readonly position: Promise<LogPosition> } | undefined;

    /** Watch the commits announced on a channel, or only this connection's without one. */
    constructor(
        channel: Channel<Commit> | undefined,
        announced: ReadonlySet<string>,
        tables: readonly string[],
    ) {
        // store the channel, the tables the database announces and the declared tables
        this.#channel = channel;
        this.#announced = announced;
        this.#tables = tables;

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

    /** Whether readers wait for a commit. */
    get isWaiting(): boolean {
        return this.#waiting.size > 0;
    }

    /** Wake a commit's readers and announce it, except the tables the database announces itself. */
    notify(tables: readonly string[]): void {
        // wake every reader of a sole writer's commit
        if (this.#channel === undefined) {
            this.#wake(tables);
        }
        // wake the readers of the tables the connection announces, and announce them to the others
        else {
            const unannounced = tables.filter((table) => !this.#announced.has(table));
            if (unannounced.length > 0) {
                this.#wake(unannounced);
                this.#channel.notify({ kind: "commit", tables: unannounced });
            }
        }
    }

    /** Fail every waiting and later reader. */
    fail(error: unknown): void {
        // record the failure, end the listening wait and wake the readers
        this.#failure = { error };
        this.#failListening(error);
        const waiting = [...this.#waiting];
        this.#waiting.clear();
        for (const wake of waiting) {
            wake(error);
        }
    }

    /** Read the log head once per commit of a sole writer, which hears each commit as it ends. */
    head(read: () => Promise<LogPosition>): Promise<LogPosition> {
        // read the head of a database other writers commit to as well
        if (this.#channel !== undefined) {
            return read();
        }

        // reuse the head read since the latest commit
        if (this.#head?.generation === this.#generation) {
            return this.#head.position;
        }

        // read it once for this commit, forgetting a failed read
        const head = { generation: this.#generation, position: read() };
        this.#head = head;
        head.position.catch(() => {
            if (this.#head === head) {
                this.#head = undefined;
            }
        });

        return head.position;
    }

    /** Wait until a check of the tables committed since the last check passes after a commit. */
    async until(
        check: (tables: ReadonlySet<string>) => Promise<boolean>,
        signal: AbortSignal,
    ): Promise<boolean> {
        // collect the tables of each commit from here on
        const committed = new Set(this.#tables);
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

    /** Wake every reader and stop listening for other writers. */
    async stop(): Promise<void> {
        // end the listening wait and wake every reader
        this.#settleListening();
        this.#wake(this.#tables);
        await this.#stop?.();
    }

    /** Wake every reader waiting for a commit, adding the committed tables to each reader's. */
    #wake(tables: readonly string[]): void {
        // count the commit
        this.#generation += 1;

        // add the tables
        for (const committed of this.#committed) {
            for (const table of tables) {
                committed.add(table);
            }
        }

        // wake the readers
        const waiting = [...this.#waiting];
        this.#waiting.clear();
        for (const wake of waiting) {
            wake();
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
        const stop = this.#channel?.listen(
            (message) => {
                this.#sequence = Math.max(this.#sequence, message.sequence ?? 0);
                this.#wake(message.tables);
            },
            () => {
                this.#settleListening();
                this.#wake(this.#tables);
            },
            (error) => this.fail(error),
        );

        return async () => stop?.();
    }
}
