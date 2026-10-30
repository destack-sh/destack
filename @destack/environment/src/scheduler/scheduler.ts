import { yieldToEventLoop } from "../clock/wait.ts";

/** Who finishes tracked work: the world, the host, or another party. */
export type Finisher = "world" | "host" | "peer";

/** The scheduler work runs under, which a world's loop reads to know when it is idle. */
export interface Scheduler {
    /** Let the host's event loop run its waiting tasks once, as the web's `scheduler.yield()`. */
    readonly yield: () => Promise<void>;
    /** Count work in flight until it settles, by who finishes it. */
    readonly track: <Value>(work: Promise<Value>, by: Finisher) => Promise<Value>;
}

/** Schedulers. */
export const Scheduler: { readonly system: Scheduler } = {
    /** The system's scheduler, counting nothing. */
    system: {
        yield: yieldToEventLoop,
        track: (work) => work,
    },
};
