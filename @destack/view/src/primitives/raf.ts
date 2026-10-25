import { isServer } from "@solidjs/web";
import { type Accessor, createMemo, createSignal, onCleanup, untrack } from "solid-js";
import { access, callbackOf, type MaybeAccessor, TRANSPARENT } from "./utils.ts";
import { useEnvironment } from "../page/view.ts";
import type { Display } from "@destack/environment/browser";

/** The milliseconds since a frame counter started, with its controls. */
export type MsCounter = Accessor<number> & {
    /** Count from zero again at the next frame. */
    readonly reset: () => void;
    /** Whether the counter runs. */
    readonly running: Accessor<boolean>;
    /** Start counting. */
    readonly start: () => void;
    /** Stop counting. */
    readonly stop: () => void;
};

/** The milliseconds in a second, which a frame rate divides into an interval. */
const SECOND = 1000;

/** Run a callback every animation frame while started, until cleanup. */
export function createRAF(
    callback: FrameRequestCallback,
): [running: Accessor<boolean>, start: () => void, stop: () => void] {
    // run nothing on the server
    if (isServer) {
        return [() => false, () => {}, () => {}];
    }

    // loop over the frames of the page's display while running
    const { display } = useEnvironment();
    const [running, setRunning] = createSignal(false, { ownedWrite: true });
    let cancel: (() => void) | undefined;
    const start = (): void => {
        if (!untrack(running)) {
            setRunning(true);
            cancel = loopFrames(callback, display);
        }
    };
    const stop = (): void => {
        setRunning(false);
        cancel?.();
    };
    onCleanup(stop);

    return [running, start, stop];
}

/** Wrap a frame callback to run at most a number of times a second. */
export function targetFPS(
    callback: FrameRequestCallback,
    fps: MaybeAccessor<number>,
): FrameRequestCallback {
    // keep the callback as it is on the server
    if (isServer) {
        return callback;
    }

    // follow the interval a reactive frame rate gives, or hold a fixed one's
    let interval: Accessor<number>;
    if (typeof fps === "function") {
        interval = createMemo(() => Math.floor(SECOND / fps()), TRANSPARENT);
    } else {
        const fixed = Math.floor(SECOND / fps);
        interval = () => fixed;
    }

    // run once the interval has passed, counting what the last run overshot
    let lastRun = 0;
    let missedBy = 0;

    return (time) => {
        const elapsed = time - lastRun;
        const wait = interval();
        if (Math.ceil(elapsed + missedBy) >= wait) {
            lastRun = time;
            missedBy = Math.max(elapsed - wait, 0);
            callback(time);
        }
    };
}

/** Count the milliseconds since the first frame at a frame rate. */
export function createMs(fps: MaybeAccessor<number>, limit?: MaybeAccessor<number>): MsCounter {
    // count from the first frame, starting over at the limit
    const [ms, setMs] = createSignal(0, { ownedWrite: true });
    let initial = 0;
    const reset = (): void => {
        initial = 0;
    };
    const [running, start, stop] = createRAF(
        targetFPS((time) => {
            // count from the first frame
            initial ||= time;
            const elapsed = time - initial;
            setMs(elapsed);

            // start over once the limit is reached
            if (limit !== undefined && elapsed >= access(limit)) {
                reset();
            }
        }, fps),
    );
    start();

    return Object.assign(ms, { reset, running, start, stop });
}

/** Call back on a display's next frame, returning a cancel, as `requestAnimationFrame` does. */
export function requestFrame(callback: (time: number) => void, display: Display): () => void {
    return callbackOf((signal) => display.frame(signal), callback);
}

/** Call back on each frame of a display until cancelled, returning a cancel function. */
export function loopFrames(callback: (time: number) => void, display: Display): () => void {
    // run each frame's callback, then request the next unless the callback cancelled the loop
    const loop = new AbortController();
    let cancelFrame: (() => void) | undefined;
    const next = (): void => {
        cancelFrame = requestFrame((time) => {
            callback(time);
            if (!loop.signal.aborted) {
                next();
            }
        }, display);
    };
    next();

    return () => {
        loop.abort();
        cancelFrame?.();
    };
}
