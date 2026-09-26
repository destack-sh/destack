import type { CommitNotifier } from "./notifier.ts";
import type { Log } from "./log.ts";

/** The commits one physical connection watches for: its readers wait for the next, which its own writes and the notifier announce. */
export class CommitWatch {
    /** The notifications exchanged with the other writers. */
    readonly #notifier: CommitNotifier;
    /** The readers waiting for the next commit. */
    readonly #waiting = new Set<(failure?: unknown) => void>();
    /** Stop listening for other writers' commits, once listening started. */
    #stop?: () => Promise<void>;
    /** The failure that ended listening, which every later wait reports. */
    #failure?: { readonly error: unknown };

    /** Watch commits, listening for other writers' once a reader waits. */
    constructor(notifier: CommitNotifier) {
        this.#notifier = notifier;
    }

    /** Whether readers wait for a commit, which polling notifiers check before reading. */
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

    /** Announce a commit this connection made: wake its own readers and notify the other writers. */
    notify(): void {
        this.wake();
        this.#notifier.notify();
    }

    /** Fail every waiting and later reader, since no notification reaches them any more. */
    fail(error: unknown): void {
        // record the failure and wake the waiting readers with it
        this.#failure = { error };
        const waiting = [...this.#waiting];
        this.#waiting.clear();
        for (const wake of waiting) {
            wake(error);
        }
    }

    /** Wait until a check of the log holds, checking again after each commit; false once the signal aborts. */
    async until(log: Log, check: () => Promise<boolean>, signal: AbortSignal): Promise<boolean> {
        while (!signal.aborted) {
            // register for the next commit before checking, so no commit goes unnoticed
            const checked = new AbortController();
            const next = this.#next(log, AbortSignal.any([signal, checked.signal]));
            if (await check()) {
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

    /** Wait for the next commit to a log, or until the signal aborts. */
    #next(log: Log, signal: AbortSignal): Promise<void> {
        // report a failed notifier, and listen for other writers from the first wait on
        if (this.#failure) {
            return Promise.reject(this.#failure.error);
        }
        this.#stop ??= this.#notifier.listen(this, log);

        return new Promise((resolve, reject) => {
            // end at once when already aborted
            if (signal.aborted) {
                resolve();

                return;
            }

            // end at the next commit, the notifier's failure or the abort, whichever comes first
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
}
