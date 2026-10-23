import type { Clock } from "../clock/clock.ts";
import { SYSTEM } from "./global.ts";

/** A wait on a virtual clock, released once the clock reaches its deadline. */
interface VirtualSleep {
    /** When the wait ends, in UTC epoch milliseconds. */
    readonly deadline: number;
    /** Release the wait. */
    readonly release: () => void;
    /** Whether the wait keeps the process running: a sleep does, a timeout signal does not. */
    readonly isHeld: boolean;
}

/** A clock that a test pauses, moves and resumes. */
export class VirtualClock implements Clock {
    /** The clock's reading when it paused or last moved, in UTC epoch milliseconds. */
    #time: number;
    /** The system's time when the clock resumed or last moved, absent while it is paused. */
    #resumedAt: number | undefined;
    /** The system timer that wakes the earliest wait while the clock flows. */
    #timer: ReturnType<typeof setTimeout> | undefined;
    /** The pending waits, each released once the clock reaches its deadline. */
    readonly #sleeps: VirtualSleep[] = [];
    /** The waits begun so far. */
    #begun = 0;
    /** The stops of the pending timers, by handle. */
    readonly #timers = new Map<number, AbortController>();
    /** The next timer handle. */
    #handle = 1;

    /** Start the clock paused at a time. */
    constructor(time: number) {
        this.#time = time;
    }

    /** Read the clock's time. */
    now(): number {
        return this.#resumedAt === undefined
            ? this.#time
            : this.#time + SYSTEM.Date.now() - this.#resumedAt;
    }

    /** Wait some milliseconds from now, rounded up to whole ones, until the clock passes them. */
    sleep(milliseconds: number, signal?: AbortSignal): Promise<void> {
        this.#begun += 1;

        return this.#wait(milliseconds, true, signal);
    }

    /** Make a signal aborting with a timeout once the clock passes some milliseconds from now. */
    timeout(milliseconds: number): AbortSignal {
        // abort as the web's timeout signal does, outside the counted waits
        const controller = new AbortController();
        void this.#wait(milliseconds, false).then(() =>
            controller.abort(new DOMException("signal timed out", "TimeoutError")),
        );

        return controller.signal;
    }

    /** Add a wait until the clock passes some milliseconds from now. */
    #wait(milliseconds: number, isHeld: boolean, signal?: AbortSignal): Promise<void> {
        // refuse aborted waits, end empty waits
        if (signal?.aborted === true) {
            return Promise.reject(signal.reason);
        } else if (milliseconds <= 0) {
            return Promise.resolve();
        }

        // add the wait until its deadline
        const { promise, resolve, reject } = Promise.withResolvers<void>();
        const sleep: VirtualSleep = {
            deadline: this.now() + Math.ceil(milliseconds),
            release: () => {
                signal?.removeEventListener("abort", abort);
                resolve();
            },
            isHeld,
        };
        const abort = () => {
            this.#sleeps.splice(this.#sleeps.indexOf(sleep), 1);
            this.#follow();
            reject(signal?.reason);
        };
        this.#sleeps.push(sleep);
        signal?.addEventListener("abort", abort, { once: true });

        // wake it on time while the clock flows
        this.#follow();

        return promise;
    }

    /** The waits begun so far. */
    get begun(): number {
        return this.#begun;
    }

    /** The earliest deadline of a pending wait, absent with none. */
    get next(): number | undefined {
        return this.#sleeps.length === 0
            ? undefined
            : Math.min(...this.#sleeps.map((sleep) => sleep.deadline));
    }

    /** Let the clock flow with the system's time from its current reading. */
    resume(): this {
        if (this.#resumedAt === undefined) {
            this.#resumedAt = SYSTEM.Date.now();
            this.#follow();
        }

        return this;
    }

    /** Stop the clock at its current reading. */
    pause(): void {
        this.#time = this.now();
        this.#resumedAt = undefined;
        this.#follow();
    }

    /** Move the clock to a later time, releasing the waits it passes, earliest first. */
    set(time: number): void {
        // refuse moving back, and a time between whole milliseconds
        const now = this.now();
        if (time < now) {
            throw new RangeError(`a virtual clock at ${now} cannot go back to ${time}`);
        } else if (!Number.isInteger(time)) {
            throw new RangeError(`a virtual clock moves in whole milliseconds, not to ${time}`);
        }

        // move the reading, keeping a flowing clock flowing
        this.#time = time;
        if (this.#resumedAt !== undefined) {
            this.#resumedAt = SYSTEM.Date.now();
        }
        this.#wake();
    }

    /** Move the clock forward some milliseconds, releasing the waits it passes. */
    advance(milliseconds: number): void {
        this.set(this.now() + milliseconds);
    }

    /** Run a function once some milliseconds of the clock passed, answering the timer's handle. */
    setTimeout(handler: unknown, delay = 0, ...values: unknown[]): number {
        return this.#schedule(handler, delay, values, false);
    }

    /** Run a function each period of the clock, answering the timer's handle. */
    setInterval(handler: unknown, delay = 0, ...values: unknown[]): number {
        return this.#schedule(handler, delay, values, true);
    }

    /** Stop a timer. */
    clearTimeout(handle?: number): void {
        if (handle !== undefined) {
            this.#timers.get(handle)?.abort();
            this.#timers.delete(handle);
        }
    }

    /** Stop an interval. */
    clearInterval(handle?: number): void {
        this.clearTimeout(handle);
    }

    /** Start a timer under a new handle. */
    #schedule(handler: unknown, delay: number, values: unknown[], isRepeating: boolean): number {
        // refuse code strings
        if (typeof handler !== "function") {
            throw new TypeError("a virtual timer can only run functions");
        }

        // add the timer under a new handle
        const handle = this.#handle;
        this.#handle += 1;
        const stop = new AbortController();
        this.#timers.set(handle, stop);

        // run a timer of no time on the system's next turn, and a later one on the clock
        if (delay <= 0 && !isRepeating) {
            SYSTEM.setTimeout(() => {
                if (!stop.signal.aborted) {
                    this.#timers.delete(handle);
                    Reflect.apply(handler, undefined, values);
                }
            }, 0);
        } else {
            this.#arm(handle, handler, Math.max(delay, 1), values, isRepeating, stop.signal);
        }

        return handle;
    }

    /** Wait one period of a timer, then run it. */
    #arm(
        handle: number,
        handler: Function,
        delay: number,
        values: unknown[],
        isRepeating: boolean,
        signal: AbortSignal,
    ): void {
        this.sleep(delay, signal).then(
            () => {
                // wait again for an interval
                if (isRepeating) {
                    this.#arm(handle, handler, delay, values, isRepeating, signal);
                }
                // remove a single timer
                else {
                    this.#timers.delete(handle);
                }
                Reflect.apply(handler, undefined, values);
            },
            () => {},
        );
    }

    /** Release the waits the clock reached, earliest first. */
    #wake(): void {
        // release the due waits in deadline order
        const now = this.now();
        const due = this.#sleeps
            .filter((sleep) => sleep.deadline <= now)
            .toSorted((left, right) => left.deadline - right.deadline);
        for (const sleep of due) {
            this.#sleeps.splice(this.#sleeps.indexOf(sleep), 1);
            sleep.release();
        }

        // follow the next wait
        this.#follow();
    }

    /** Set a system timer for the earliest wait while the clock flows. */
    #follow(): void {
        // clear the earlier timer
        SYSTEM.clearTimeout(this.#timer);
        this.#timer = undefined;

        // wake at the earliest deadline while flowing, keeping the process running for held waits alone
        const next = this.next;
        if (this.#resumedAt !== undefined && next !== undefined) {
            const timer = SYSTEM.setTimeout(() => this.#wake(), Math.max(next - this.now(), 0));
            if (!this.#sleeps.some((sleep) => sleep.isHeld)) {
                timer.unref();
            }
            this.#timer = timer;
        }
    }
}
