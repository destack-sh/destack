import type { Channel, Commit } from "../channel/channel.ts";

/** Who announces a connection's commits on its channel: the connection once committed, or the database inside the commit. */
export type Announcer = "connection" | "database";

/** The commits one physical connection's readers wait for. */
export class CommitWatch {
    /** The channel other writers announce their commits on, absent for a sole writer. */
    readonly #channel: Channel<Commit> | undefined;
    /** The SQL names of the tables whose commits the database announces itself, inside the commit. */
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

    /** Watch the commits announced on a channel, or only this connection's without one. */
    constructor(
        channel: Channel<Commit> | undefined,
        announced: ReadonlySet<string>,
        tables: readonly string[],
    ) {
        // keep the channel, the tables the database announces and the declared tables
        this.#channel = channel;
        this.#announced = announced;
        this.#tables = tables;
    }

    /** Whether readers wait for a commit. */
    get isWaiting(): boolean {
        return this.#waiting.size > 0;
    }

    /** Wake this connection's readers and announce the commit of the tables the database leaves unannounced to the other writers. */
    notify(tables: readonly string[]): void {
        this.#wake(tables);
        const unannounced = tables.filter((table) => !this.#announced.has(table));
        if (unannounced.length > 0) {
            this.#channel?.notify({ kind: "commit", tables: unannounced });
        }
    }

    /** Fail every waiting and later reader. */
    fail(error: unknown): void {
        // record the failure and wake the readers
        this.#failure = { error };
        const waiting = [...this.#waiting];
        this.#waiting.clear();
        for (const wake of waiting) {
            wake(error);
        }
    }

    /**
     * Wait until a check of the tables committed since the last check passes after a commit, returning false once the signal aborts.
     *
     * The first check gets every table.
     */
    async until(
        check: (tables: ReadonlySet<string>) => Promise<boolean>,
        signal: AbortSignal,
    ): Promise<boolean> {
        // collect the tables of each commit from here on
        const committed = new Set(this.#tables);
        this.#committed.add(committed);
        try {
            while (!signal.aborted) {
                // register before checking, and hand the check the tables committed since the last one
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

    /** Wake every reader and stop listening for other writers. */
    async stop(): Promise<void> {
        this.#wake(this.#tables);
        await this.#stop?.();
    }

    /** Wake every reader waiting for a commit, adding the committed tables to each reader's. */
    #wake(tables: readonly string[]): void {
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

    /** Wait for the next commit or the abort. */
    #next(signal: AbortSignal): Promise<void> {
        // report a failure and start listening
        if (this.#failure) {
            return Promise.reject(this.#failure.error);
        }
        this.#stop ??= this.#listen();

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

    /** Wake the readers on each commit the channel announces, and on each resumed delivery as a commit of every table. */
    #listen(): () => Promise<void> {
        const stop = this.#channel?.listen(
            (message) => this.#wake(message.tables),
            () => this.#wake(this.#tables),
            (error) => this.fail(error),
        );

        return async () => stop?.();
    }
}
