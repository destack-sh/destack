import type { Channel, Commit } from "../channel/channel.ts";

/** Who announces a connection's commits on its channel: the connection once committed, or the database inside the commit. */
export type Announcer = "connection" | "database";

/** The commits one physical connection's readers wait for. */
export class CommitWatch {
    /** The channel other writers announce their commits on, absent for a sole writer. */
    readonly #channel: Channel<Commit> | undefined;
    /** Who announces this connection's commits. */
    readonly #announcer: Announcer;
    /** The readers waiting for the next commit. */
    readonly #waiting = new Set<(failure?: unknown) => void>();
    /** Stop listening for other writers' commits. */
    #stop?: () => Promise<void>;
    /** The failure that ended listening. */
    #failure?: { readonly error: unknown };

    /** Watch the commits announced on a channel, or only this connection's without one. */
    constructor(channel?: Channel<Commit>, announcer: Announcer = "connection") {
        this.#channel = channel;
        this.#announcer = announcer;
    }

    /** Whether readers wait for a commit. */
    get isWaiting(): boolean {
        return this.#waiting.size > 0;
    }

    /** Wake every reader waiting for a commit. */
    wake(): void {
        const waiting = [...this.#waiting];
        this.#waiting.clear();
        for (const wake of waiting) {
            wake();
        }
    }

    /** Wake this connection's readers and announce the commit to the other writers. */
    notify(): void {
        this.wake();
        if (this.#announcer === "connection") {
            this.#channel?.notify({ kind: "commit" });
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

    /** Wait until a check holds after a commit, returning false once the signal aborts. */
    async until(check: () => Promise<boolean>, signal: AbortSignal): Promise<boolean> {
        while (!signal.aborted) {
            // register before checking
            const checked = new AbortController();
            const next = this.#next(AbortSignal.any([signal, checked.signal]));
            const isHeld = await check().catch(async (error: unknown) => {
                // settle the registered wait
                checked.abort();
                await Promise.allSettled([next]);
                throw error;
            });
            if (isHeld) {
                checked.abort();
                await next;

                return true;
            }
            await next;
        }

        return false;
    }

    /** Wake every reader and stop listening for other writers. */
    async stop(): Promise<void> {
        this.wake();
        await this.#stop?.();
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

    /** Wake the readers on each commit the channel announces and on each resumed delivery. */
    #listen(): () => Promise<void> {
        const stop = this.#channel?.listen(
            (message) => {
                if (message.kind === "commit") {
                    this.wake();
                }
            },
            () => this.wake(),
            (error) => this.fail(error),
        );

        return async () => stop?.();
    }
}
