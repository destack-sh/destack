/** A source of the current time, one per server, which its verbs read, wait on and tests set. */
export interface Clock {
    /** Read the current time, in UTC epoch milliseconds. */
    now(): number;
    /** Wait some milliseconds of this clock, rejecting with the signal's reason once it aborts. */
    sleep(milliseconds: number, signal?: AbortSignal): Promise<void>;
}

/** The longest timer delay, the largest signed 32 bit integer, in milliseconds. */
export const LONGEST_DELAY = 2 ** 31 - 1;

/** The clocks a server runs on. */
export const Clock: { readonly system: Clock } = {
    /** The system's clock, waiting on real timers. */
    system: {
        now: () => Date.now(),
        sleep: (milliseconds, signal) =>
            new Promise<void>((resolve, reject) => {
                // refuse a wait already aborted
                if (signal?.aborted === true) {
                    reject(signal.reason);

                    return;
                }

                // wait one timer at a time until the time passes, or reject once the signal aborts
                let remaining = milliseconds;
                let timer: ReturnType<typeof setTimeout> | undefined;
                const next = () => {
                    const step = Math.min(Math.max(remaining, 0), LONGEST_DELAY);
                    remaining -= step;
                    timer = setTimeout(() => {
                        if (remaining > 0) {
                            next();
                        } else {
                            signal?.removeEventListener("abort", abort);
                            resolve();
                        }
                    }, step);
                };
                const abort = () => {
                    clearTimeout(timer);
                    reject(signal?.reason);
                };
                signal?.addEventListener("abort", abort, { once: true });
                next();
            }),
    },
};
