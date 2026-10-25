/** The longest idle period in milliseconds (HTML Standard). */
const LONGEST_IDLE_MILLISECONDS = 50;

/** A browser's display: its frames, its idle periods and its media features. */
export interface Display {
    /** Wait for the next frame, as `requestAnimationFrame` does, answering its time. */
    readonly frame: (signal?: AbortSignal) => Promise<number>;
    /** Wait for an idle period, at most a timeout, as `requestIdleCallback` does. */
    readonly idle: (timeout?: number, signal?: AbortSignal) => Promise<void>;
    /** Follow a media query, as `matchMedia` does. */
    readonly media: (query: string) => MediaQueryList;
}

/** Displays. */
export const Display: { readonly system: Display } = {
    /** The browser's display. */
    system: {
        frame: (signal) =>
            new Promise((resolve, reject) => {
                // request the frame, forgetting the signal once it came
                signal?.throwIfAborted();
                const listening = new AbortController();
                const request = requestAnimationFrame((time) => {
                    listening.abort();
                    resolve(time);
                });

                // cancel it once the signal aborts first
                const abort = () => {
                    cancelAnimationFrame(request);
                    reject(signal?.reason);
                };
                signal?.addEventListener("abort", abort, { once: true, signal: listening.signal });
            }),
        idle: (timeout, signal) =>
            new Promise((resolve, reject) => {
                // request the idle period, or a task soon in Safari, which has no idle callbacks
                signal?.throwIfAborted();
                const listening = new AbortController();
                const done = () => {
                    listening.abort();
                    resolve();
                };
                const request =
                    typeof requestIdleCallback === "function"
                        ? {
                              idle: requestIdleCallback(
                                  done,
                                  timeout === undefined ? {} : { timeout },
                              ),
                          }
                        : {
                              timer: setTimeout(
                                  done,
                                  Math.min(
                                      timeout ?? LONGEST_IDLE_MILLISECONDS,
                                      LONGEST_IDLE_MILLISECONDS,
                                  ),
                              ),
                          };

                // cancel it once the signal aborts first
                const abort = () => {
                    if ("idle" in request) {
                        cancelIdleCallback(request.idle);
                    } else {
                        clearTimeout(request.timer);
                    }
                    reject(signal?.reason);
                };
                signal?.addEventListener("abort", abort, { once: true, signal: listening.signal });
            }),
        media: (query) => matchMedia(query),
    },
};
