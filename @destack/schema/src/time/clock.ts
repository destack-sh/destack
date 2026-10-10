/** A source of the current time, one per server, which its verbs read and tests set. */
export interface Clock {
    /** Read the current time, in UTC epoch milliseconds. */
    now(): number;
}

/** The clocks a server runs on. */
export const Clock = {
    /** The system's clock. */
    system: { now: () => Date.now() } satisfies Clock,
};
