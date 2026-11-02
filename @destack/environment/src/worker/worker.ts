/** A running worker thread, as the platform's `Worker` has it. */
export interface WorkerThread {
    /** Send the worker a message. */
    readonly postMessage: (message: unknown) => void;
    /** Take each message the worker sends. */
    readonly addEventListener: (type: "message", listener: (event: MessageEvent) => void) => void;
    /** Stop the worker at once. */
    readonly terminate: () => void;
}

/** A constructor of module workers, as the platform's `Worker` has it. */
export type WorkerConstructor = new (
    url: URL,
    options: { readonly type: "module"; readonly name?: string },
) => WorkerThread;

/** The host of worker threads, handing its worker constructor to the code starting one. */
export interface WorkerHost {
    /** Start a worker as `new Worker(new URL(module, import.meta.url))`, which builds bundle. */
    readonly start: (create: (Worker: WorkerConstructor) => WorkerThread) => WorkerThread;
}

/** Worker hosts. */
export const WorkerHost = {
    /** The platform's module workers. */
    system: { start: (create) => create(Worker) } satisfies WorkerHost,
};
