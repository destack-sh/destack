/** How a worker thread starts. */
export interface WorkerOptions {
    /** The worker's name, as debuggers show it. */
    readonly name?: string;
}

/** A running worker thread, as the platform's `Worker` has it. */
export interface WorkerThread {
    /** Send the worker a message. */
    readonly postMessage: (message: unknown) => void;
    /** Take each message the worker sends. */
    readonly addEventListener: (type: "message", listener: (event: MessageEvent) => void) => void;
    /** Stop the worker at once. */
    readonly terminate: () => void;
}

/** The host of worker threads, which starts them from module scripts. */
export interface WorkerHost {
    /** Start a worker thread from a module script. */
    readonly start: (url: URL, options?: WorkerOptions) => WorkerThread;
}

/** Worker hosts. */
export const WorkerHost = {
    /** The platform's module workers. */
    system: {
        start: (url, options = {}) => new Worker(url, { ...options, type: "module" }),
    } satisfies WorkerHost,
};
