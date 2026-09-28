/** The longest delay a runtime timer accepts, the largest signed 32 bit integer, in milliseconds. */
export const MAX_TIMER_DELAY = 2 ** 31 - 1;

/**
 * Wait a delay in milliseconds, as the web platform's `scheduler.wait` does, rejecting with the signal's reason once it aborts.
 *
 * Runtimes without `scheduler.wait`, such as Bun and Workers, get this in its place.
 */
export function wait(
    delay: number,
    options: { readonly signal?: AbortSignal } = {},
): Promise<void> {
    const signal = options.signal;

    return new Promise((resolve, reject) => {
        // reject at once for an aborted signal
        if (signal?.aborted) {
            reject(signal.reason);

            return;
        }

        // resolve after the delay, or reject on abort, whichever comes first
        const timer = setTimeout(() => {
            signal?.removeEventListener("abort", abort);
            resolve();
        }, delay);
        function abort() {
            clearTimeout(timer);
            reject(signal!.reason);
        }
        signal?.addEventListener("abort", abort, { once: true });
    });
}
