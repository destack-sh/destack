import type { Environment, HostCapability } from "../environment/environment.ts";
import type { Scheduler } from "../scheduler/scheduler.ts";
import type { WorkerConstructor, WorkerEvents, WorkerHost, WorkerThread } from "./worker.ts";

/** The scope a worker's body runs in: its messages to and from the node that started it. */
export interface WorkerScope {
    /** The worker's name. */
    readonly name: string;
    /** Send the starting node a message. */
    readonly postMessage: (message: unknown) => void;
    /** Take each message the starting node sends. */
    readonly addEventListener: (type: "message", listener: (event: MessageEvent) => void) => void;
}

/** What a worker's module does in a world, given its scope and its node's environment. */
export type WorkerBody = (
    scope: WorkerScope,
    environment: Environment<HostCapability>,
    signal: AbortSignal,
) => void | Promise<void>;

/** A world's host of worker threads: each a node running a body defined for its module. */
export class SimulatedWorkerHost {
    /** The bodies by module URL. */
    readonly #bodies = new Map<string, WorkerBody>();
    /** The scheduler each message's delivery counts on. */
    readonly #scheduler: Scheduler;
    /** Start a body on a node of the world, stopped once the node crashes. */
    readonly #start: (
        node: string,
        body: (environment: Environment<HostCapability>, signal: AbortSignal) => Promise<void>,
    ) => void;
    /** Crash a worker's node, as terminating it does. */
    readonly #crash: (node: string) => void;
    /** The workers started so far, which number their nodes. */
    #started = 0;
    /** The terminations of each node's running workers, by the node that started them. */
    readonly #running = new Map<string, Set<() => void>>();

    /** Run workers as nodes of a world, delivering their messages on its scheduler. */
    constructor(options: {
        readonly scheduler: Scheduler;
        readonly start: (
            node: string,
            body: (environment: Environment<HostCapability>, signal: AbortSignal) => Promise<void>,
        ) => void;
        readonly crash: (node: string) => void;
    }) {
        this.#scheduler = options.scheduler;
        this.#start = options.start;
        this.#crash = options.crash;
    }

    /** Define what a module's workers do in this world. */
    define(url: URL, body: WorkerBody): void {
        this.#bodies.set(url.href, body);
    }

    /** Terminate the workers a node started, as its crash ends them with its process. */
    release(parent: string): void {
        for (const terminate of this.#running.get(parent) ?? []) {
            terminate();
        }
        this.#running.delete(parent);
    }

    /** Read a node's workers, each started on a separate node. */
    of(parent: string): WorkerHost {
        return {
            start: (create) =>
                create(workerConstructor((url, name) => this.#thread(parent, url, name))),
        };
    }

    /** Start a worker of a module on a separate node, refusing a module with no body. */
    #thread(parent: string, url: URL, name: string | undefined): WorkerThread {
        // refuse a module this world defines no body for
        const body = this.#bodies.get(url.href);
        if (body === undefined) {
            throw new TypeError(`this world defines no worker body for '${url.href}'`);
        }

        // name the worker's node after its parent and its order
        this.#started += 1;
        const node = `${parent}/worker-${this.#started}`;
        const toWorker = new Set<(event: MessageEvent) => void>();
        const toParent: { [Type in keyof WorkerEvents]: Set<(event: WorkerEvents[Type]) => void> } =
            {
                message: new Set(),
                error: new Set(),
            };
        let isTerminated = false;

        // run the body on its node with a scope sending to the parent
        const scope: WorkerScope = {
            name: name ?? url.href,
            postMessage: (message) => this.#deliver(toParent.message, message, () => isTerminated),
            addEventListener: (_type, listener) => toWorker.add(listener),
        };
        this.#start(node, async (environment, signal) => {
            // tell the parent of a failure as later work, surfacing one no parent listens for
            try {
                await body(scope, environment, signal);
            } catch (error) {
                if (!signal.aborted) {
                    await this.#scheduler.task(() =>
                        Promise.resolve().then(() => {
                            const message = error instanceof Error ? error.message : String(error);
                            if (toParent.error.size === 0) {
                                throw error;
                            }
                            for (const listener of toParent.error) {
                                listener({ message });
                            }
                        }),
                    );
                }
            }
        });

        // terminate it once asked, or once its parent crashes
        const terminate = () => {
            isTerminated = true;
            this.#running.get(parent)?.delete(terminate);
            this.#crash(node);
        };
        const running = this.#running.get(parent) ?? new Set();
        running.add(terminate);
        this.#running.set(parent, running);

        return {
            postMessage: (message) => this.#deliver(toWorker, message, () => isTerminated),
            addEventListener: (type, listener) => toParent[type].add(listener),
            terminate,
        };
    }

    /** Deliver a copy of a message after this turn as the world's work, unless terminated. */
    #deliver(
        listeners: ReadonlySet<(event: MessageEvent) => void>,
        message: unknown,
        isTerminated: () => boolean,
    ): void {
        const data: unknown = structuredClone(message);
        void this.#scheduler.task(() =>
            Promise.resolve().then(() => {
                // drop it once the worker terminated, as a terminated worker's port does
                if (isTerminated()) {
                    return;
                }

                // call each listener with the copy
                const event = new MessageEvent("message", { data });
                for (const listener of listeners) {
                    listener(event);
                }
            }),
        );
    }
}

/** Make a worker constructor whose workers are the threads a start function answers. */
function workerConstructor(
    start: (url: URL, name: string | undefined) => WorkerThread,
): WorkerConstructor {
    return class implements WorkerThread {
        /** The thread the start function answered. */
        readonly #thread: WorkerThread;

        /** Start a module's worker. */
        constructor(url: URL, options: { readonly name?: string }) {
            this.#thread = start(url, options.name);
        }

        /** Send the worker a message. */
        postMessage(message: unknown): void {
            this.#thread.postMessage(message);
        }

        /** Take each event of a type the worker sends. */
        addEventListener<Type extends keyof WorkerEvents>(
            type: Type,
            listener: (event: WorkerEvents[Type]) => void,
        ): void {
            this.#thread.addEventListener(type, listener);
        }

        /** Stop the worker at once. */
        terminate(): void {
            this.#thread.terminate();
        }
    };
}
