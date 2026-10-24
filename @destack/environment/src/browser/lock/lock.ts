/** How a lock is held: alone, or beside other shared holders (Web LockManager API). */
export type LockMode = "exclusive" | "shared";

/** How a lock is requested. */
export interface LockOptions {
    /** How the lock is held, exclusive by default. */
    readonly mode?: LockMode;
    /** Whether to run the callback without the lock at once when another has it. */
    readonly ifAvailable?: boolean;
    /** Give up waiting for the lock once it aborts. */
    readonly signal?: AbortSignal;
}

/** A held lock's name and mode, absent for a request that found it taken. */
export interface HeldLock {
    /** The lock's name. */
    readonly name: string;
    /** How it is held. */
    readonly mode: LockMode;
}

/** The origin's named locks every tab and worker shares, as the Web LockManager API has them. */
export interface LockManager {
    /** Take a named lock while a callback runs, answering the callback's result. */
    readonly request: <Value>(
        name: string,
        options: LockOptions,
        callback: (lock: HeldLock | null) => Promise<Value>,
    ) => Promise<Value>;
}

/** Lock managers. */
export const LockManager: { readonly system: LockManager } = {
    /** The browser's lock manager. */
    system: {
        request: (name, options, callback) => navigator.locks.request(name, options, callback),
    },
};
