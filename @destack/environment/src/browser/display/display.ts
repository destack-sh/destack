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
                // request the frame, cancelling it once the signal aborts
                signal?.throwIfAborted();
                const request = requestAnimationFrame(resolve);
                signal?.addEventListener(
                    "abort",
                    () => {
                        cancelAnimationFrame(request);
                        reject(signal.reason);
                    },
                    { once: true },
                );
            }),
        idle: (timeout, signal) =>
            new Promise((resolve, reject) => {
                // request the idle period, cancelling it once the signal aborts
                signal?.throwIfAborted();
                const request = requestIdleCallback(
                    () => resolve(),
                    timeout === undefined ? {} : { timeout },
                );
                signal?.addEventListener(
                    "abort",
                    () => {
                        cancelIdleCallback(request);
                        reject(signal.reason);
                    },
                    { once: true },
                );
            }),
        media: (query) => matchMedia(query),
    },
};
