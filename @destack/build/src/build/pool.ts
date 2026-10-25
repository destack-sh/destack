import { resolve, sep } from "node:path";
import { locateWorkspace } from "@destack/check/workspace";
import { nth } from "@destack/schema";
import { Compiler, CompilerCommand } from "./compiler.ts";
import type { LocalPackageStore } from "../store/local.ts";

/** The warm compilers a machine runs at once: 4, each a TypeScript program of a few hundred MB. */
const MAX_WARM_COMPILERS = 4;

/** How long an unused compiler stays warm: fifteen minutes, since a cold start costs seconds. */
const IDLE_MILLISECONDS = 15 * 60_000;

/** A workspace's warm compiler, the package it last built and when it was last used. */
interface Warm {
    /** The compiler, started once. */
    readonly compiler: Promise<Compiler>;
    /** The workspace whose packages it builds. */
    readonly workspace: string;
    /** The package directory it last served, whose state stays warm. */
    directory: string;
    /** Whether a lease uses it now. */
    isBusy: boolean;
    /** The pool's use count at its last use, which orders reuse and eviction. */
    use: number;
    /** The abort of the timer closing the compiler once it idles. */
    idle: AbortController;
}

/** A lease waiting for a compiler of a package. */
interface Waiter {
    /** The package's workspace. */
    readonly workspace: string;
    /** The package's directory. */
    readonly directory: string;
    /** Hand the waiter its compiler. */
    readonly take: (warm: Warm) => void;
}

/** Warm compilers building into one store, each leased to one package at a time. */
export class CompilerPool implements AsyncDisposable {
    /** The store every compiler builds into. */
    readonly store: LocalPackageStore;
    /** The command the pool starts each compiler with. */
    readonly #command: CompilerCommand;
    /** The warm compilers. */
    #warm: Warm[] = [];
    /** The compilers closing, which still count against the cap. */
    #closing = 0;
    /** The uses so far, counting each compiler taken. */
    #uses = 0;
    /** The leases waiting for a compiler, first come first served. */
    readonly #waiting: Waiter[] = [];
    /** The failures of compilers closing with no lease to answer, thrown on disposal. */
    readonly #failures: unknown[] = [];

    /** Store compilers started with a command into a store. */
    private constructor(store: LocalPackageStore, command: CompilerCommand) {
        this.store = store;
        this.#command = command;
    }

    /** Open a pool starting the current compiler, building into a store. */
    static async open(store: LocalPackageStore): Promise<CompilerPool> {
        return new CompilerPool(store, await CompilerCommand.current(store.environment));
    }

    /** Run work with a package's compiler, taken for the package alone until the work settles. */
    async use<Result>(
        directory: string,
        work: (compiler: Compiler) => Promise<Result>,
    ): Promise<Result> {
        // queue for a compiler of the package's workspace
        const { files } = this.store.environment;
        const resolved = await files.realPath(resolve(directory));
        const workspace = await files.realPath(await locateWorkspace(resolved, files));
        const warm = await new Promise<Warm>((take) => {
            this.#waiting.push({ workspace, directory: resolved, take });
            this.#serve();
        });

        // run the work, then free the compiler, closing one the pool released meanwhile
        try {
            return await work(await warm.compiler);
        } finally {
            warm.isBusy = false;
            if (this.#warm.includes(warm)) {
                this.#idle(warm);
            } else {
                void this.#dispose(warm);
            }
            this.#serve();
        }
    }

    /** Hand compilers to the waiting leases in order, passing over those it cannot serve yet. */
    #serve(): void {
        let index = 0;
        while (index < this.#waiting.length) {
            // pass over a waiter no compiler serves yet, such as one whose package's compiler is busy
            const waiter = nth(this.#waiting, index);
            const warm = this.#pick(waiter);
            if (warm === undefined) {
                index += 1;
            }
            // lease the compiler to the package
            else {
                this.#waiting.splice(index, 1);
                warm.directory = waiter.directory;
                warm.isBusy = true;
                warm.use = ++this.#uses;
                this.#idle(warm);
                waiter.take(warm);
            }
        }
    }

    /** Pick a waiter's compiler: the package's, a new one, the workspace's oldest idle, or none. */
    #pick(waiter: Waiter): Warm | undefined {
        // find the package's compiler, and the idle ones by their last use
        const matching = this.#warm.find((warm) => warm.directory === waiter.directory);
        const idle = this.#warm
            .filter((warm) => !warm.isBusy)
            .toSorted((left, right) => left.use - right.use);
        const shared = idle.find((warm) => warm.workspace === waiter.workspace);
        const other = idle.find((warm) => warm.workspace !== waiter.workspace);
        const isFull = this.#warm.length + this.#closing >= MAX_WARM_COMPILERS;

        // take the package's compiler once idle, whose state is warm
        if (matching !== undefined) {
            return matching.isBusy ? undefined : matching;
        }
        // start one below the cap
        else if (!isFull) {
            return this.#start(waiter.workspace, waiter.directory);
        }
        // switch the workspace's oldest idle compiler to the package
        else if (shared !== undefined) {
            return shared;
        }
        // close another workspace's oldest idle compiler, starting this one once it stopped
        else if (other !== undefined) {
            void this.#close([other]);

            return undefined;
        }
        // wait for a compiler to come free
        else {
            return undefined;
        }
    }

    /** Start a workspace's compiler, forgetting it when it fails to start. */
    #start(workspace: string, directory: string): Warm {
        // start the compiler, closing it once it idles
        const compiler = Compiler.start(workspace, { command: this.#command, store: this.store });
        const idle = new AbortController();
        const warm: Warm = { compiler, workspace, directory, isBusy: false, use: this.#uses, idle };
        this.#warm.push(warm);
        this.#idle(warm);

        // forget one that fails to start, which its lease answers, and serve the next waiter
        compiler.catch(() => {
            warm.idle.abort();
            this.#warm = this.#warm.filter((each) => each !== warm);
            this.#serve();
        });

        return warm;
    }

    /** Close the compilers of the workspaces at or below a directory, such as a working tree. */
    async release(root: string): Promise<void> {
        const resolved = await this.store.environment.files.realPath(root);
        await this.#close(
            this.#warm.filter(
                (warm) =>
                    warm.workspace === resolved || warm.workspace.startsWith(`${resolved}${sep}`),
            ),
        );
    }

    /** Close every warm compiler, then throw the failures of compilers closed in the background. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#close(this.#warm);
        if (this.#failures.length > 0) {
            throw new AggregateError(this.#failures, "compilers failed to close");
        }
    }

    /** Forget some warm compilers, closing each idle one now and each busy one after its lease. */
    async #close(closed: readonly Warm[]): Promise<void> {
        // stop their idle timers and drop them from the pool
        for (const warm of closed) {
            warm.idle.abort();
        }
        this.#warm = this.#warm.filter((warm) => !closed.includes(warm));

        // close the idle ones, which count against the cap until they stopped
        const idle = closed.filter((warm) => !warm.isBusy);
        await Promise.all(idle.map((warm) => this.#dispose(warm)));
    }

    /** Restart a compiler's idle timer, closing it once it idles a while unused. */
    #idle(warm: Warm): void {
        // cancel the last timer and start the next
        warm.idle.abort();
        const idle = new AbortController();
        warm.idle = idle;
        void this.store.environment.clock.sleep(IDLE_MILLISECONDS, idle.signal).then(
            () => (warm.isBusy ? undefined : this.#close([warm])),
            (error: unknown) => {
                if (!idle.signal.aborted) {
                    throw error;
                }
            },
        );
    }

    /** Close a forgotten compiler, recording a failure no lease answers, then serve the waiters. */
    async #dispose(warm: Warm): Promise<void> {
        this.#closing++;
        try {
            await warm.compiler.then(
                (started) => started[Symbol.asyncDispose](),
                () => undefined,
            );
        } catch (error) {
            this.#failures.push(error);
        } finally {
            this.#closing--;
            this.#serve();
        }
    }
}
