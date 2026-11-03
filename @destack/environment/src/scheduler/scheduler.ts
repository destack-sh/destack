import { yieldToEventLoop } from "../clock/wait.ts";

/** How long real work may run without the world moving: short by default, long for slow work. */
export interface RunOptions {
    /** Whether the work may run as long as a compile or a remote request, not a file read. */
    readonly isLong?: boolean;
}

/** How a world schedules work: real operations, short or long, and its own tasks. */
export interface Scheduler {
    /** Let the host's event loop run its waiting tasks once, as the web's `scheduler.yield()`. */
    readonly yield: () => Promise<void>;
    /** Run real work as one operation, completed in the order operations began. */
    readonly run: <Value>(start: () => Promise<Value>, options?: RunOptions) => Promise<Value>;
    /** Keep the world's time until real work ends, completing it as a long operation at idle. */
    readonly waitUntil: <Value>(settled: Promise<Value>) => Promise<Value>;
    /** Run the world's own work, counted in flight until it settles. */
    readonly task: <Value>(start: () => Promise<Value>) => Promise<Value>;
}

/** Schedulers. */
export const Scheduler: { readonly system: Scheduler } = {
    /** The system's scheduler, ordering and counting nothing. */
    system: {
        yield: yieldToEventLoop,
        run: (start) => start(),
        waitUntil: (settled) => settled,
        task: (start) => start(),
    },
};
