import { yieldToEventLoop } from "../clock/wait.ts";

/** The party finishing tracked work: the world, the host, or a peer process the world waits on. */
export type Party = "world" | "host" | "peer";

/** The scheduler work runs under, which a world's loop reads to know when it is idle. */
export interface Scheduler {
    /** Let the host's event loop run its waiting tasks once, as the web's `scheduler.yield()`. */
    readonly yield: () => Promise<void>;
    /** Start work and count it in flight until it settles, by who finishes it. */
    readonly track: <Value>(start: () => Promise<Value>, by: Party) => Promise<Value>;
}

/** Schedulers. */
export const Scheduler: { readonly system: Scheduler } = {
    /** The system's scheduler, counting nothing. */
    system: {
        yield: yieldToEventLoop,
        track: (start) => start(),
    },
};
