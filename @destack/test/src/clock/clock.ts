import type { Clock } from "@destack/schema";

/** A clock that follows the system's until a test sets, freezes or advances it. */
export class TestClock implements Clock {
    /** The clock's time in UTC epoch milliseconds, absent while it follows the system's. */
    #time: number | undefined;
    /** How far the clock runs ahead of the system's while it follows it, in milliseconds. */
    #offset = 0;
    /** The pending waits, each released once the clock reaches its deadline. */
    readonly #sleeps: { readonly deadline: number; readonly release: () => void }[] = [];

    /** Start the clock at a time, following the system's clock without one. */
    constructor(time?: number) {
        this.#time = time;
    }

    /** Read the clock's time. */
    now(): number {
        return this.#time ?? Date.now() + this.#offset;
    }

    /** Wait some milliseconds from now, as the clock moves or follows the system's. */
    sleep(milliseconds: number, signal?: AbortSignal): Promise<void> {
        // refuse a wait already aborted, and end a wait of no time at once
        if (signal?.aborted === true) {
            return Promise.reject(signal.reason);
        } else if (milliseconds <= 0) {
            return Promise.resolve();
        }

        // keep the wait until the clock reaches its deadline, or the signal aborts
        const deadline = this.now() + milliseconds;
        const { promise, resolve, reject } = Promise.withResolvers<void>();
        const sleep = {
            deadline,
            release: () => {
                signal?.removeEventListener("abort", abort);
                clearTimeout(timer);
                resolve();
            },
        };
        const abort = () => {
            this.#sleeps.splice(this.#sleeps.indexOf(sleep), 1);
            clearTimeout(timer);
            reject(signal?.reason);
        };
        this.#sleeps.push(sleep);
        signal?.addEventListener("abort", abort, { once: true });

        // release it as the system's time passes while the clock follows it
        const timer =
            this.#time === undefined ? setTimeout(() => this.#wake(), milliseconds) : undefined;

        return promise;
    }

    /** Stop the clock at a time, releasing the waits it passes. */
    set(time: number): void {
        this.#time = time;
        this.#wake();
    }

    /** Stop the clock at its current reading, returning it. */
    freeze(): number {
        this.#time = this.now();

        return this.#time;
    }

    /** Move the clock forward some milliseconds, releasing the waits it passes, earliest first. */
    advance(milliseconds: number): void {
        // move a stopped clock, or run a following one further ahead of the system's
        if (this.#time === undefined) {
            this.#offset += milliseconds;
        } else {
            this.#time += milliseconds;
        }
        this.#wake();
    }

    /** Release the waits whose deadline the clock reached, earliest first. */
    #wake(): void {
        const now = this.now();
        const due = this.#sleeps
            .filter((sleep) => sleep.deadline <= now)
            .toSorted((left, right) => left.deadline - right.deadline);
        for (const sleep of due) {
            this.#sleeps.splice(this.#sleeps.indexOf(sleep), 1);
            sleep.release();
        }
    }
}
