/** The longest timer delay, the largest signed 32 bit integer, in milliseconds. */
export const MAX_TIMER_DELAY = 2 ** 31 - 1;

/** Wait a delay in milliseconds, as `scheduler.wait` does, rejecting once the signal aborts. */
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

        // wait one timer at a time until done or aborted
        let remaining = delay;
        let timer: ReturnType<typeof setTimeout>;
        const next = () => {
            const step = Math.min(remaining, MAX_TIMER_DELAY);
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
        function abort() {
            clearTimeout(timer);
            reject(signal!.reason);
        }
        signal?.addEventListener("abort", abort, { once: true });
        next();
    });
}
