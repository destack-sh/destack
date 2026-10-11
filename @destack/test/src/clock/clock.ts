import type { Clock } from "@destack/schema";

/** A clock that follows the system's until a test sets, freezes or advances it, as Go's mock clock and Java's fixed clock do. */
export class TestClock implements Clock {
    /** The time the clock stands at, in UTC epoch milliseconds, absent while it follows the system's. */
    #time: number | undefined;

    /** Start the clock at a time, following the system's clock without one. */
    constructor(time?: number) {
        this.#time = time;
    }

    /** Read the clock's time. */
    now(): number {
        return this.#time ?? Date.now();
    }

    /** Stop the clock at a time. */
    set(time: number): void {
        this.#time = time;
    }

    /** Stop the clock at its current reading, returning it. */
    freeze(): number {
        this.#time = this.now();

        return this.#time;
    }

    /** Move the clock forward by some milliseconds, stopping it there. */
    advance(milliseconds: number): void {
        this.#time = this.now() + milliseconds;
    }
}
