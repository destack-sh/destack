import { resolve, sep } from "node:path";
import { locateWorkspace } from "@destack/check/workspace";
import { nth } from "@destack/schema";
import type { Toolchain } from "@destack/check/toolchain";
import { Once } from "@destack/environment";
import { Compiler } from "./compiler.ts";
import type { LocalBuildStore } from "../store/local.ts";
import { CompilerFingerprint } from "../cache/cache.ts";
import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";

/** The warm compilers a machine runs at once: 4, each a TypeScript program of a few hundred MB. */
const MAX_WARM_COMPILERS = 4;

/** How long an unused compiler stays warm: fifteen minutes, since a cold start costs seconds. */
const IDLE_MILLISECONDS = 15 * 60_000;

/** The states a pooled compiler is in: waiting for a lease, or leased. */
const COMPILER_STATES = ["idle", "used"] as const;

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
    readonly store: LocalBuildStore;
    /** The fingerprint of the compilers, which build keys read. */
    readonly fingerprint: CompilerFingerprint;
    /** Read the toolchain the compilers run. */
    readonly #read: () => Promise<Toolchain>;
    /** The toolchain the compilers run, read on first use and again after a failure. */
    readonly #toolchain = new Once<Toolchain>();
    /** The warm compilers. */
    #warm: Warm[] = [];
    /** The closes of compilers in flight, which count against the cap and the disposal awaits. */
    readonly #closing = new Set<Promise<void>>();
    /** The leases running work, whose released compilers close once the work settles. */
    readonly #leased = new Set<Promise<void>>();
    /** The uses so far, counting each compiler taken. */
    #uses = 0;
    /** The leases waiting for a compiler, first come first served. */
    readonly #waiting: Waiter[] = [];
    /** The failures of compilers closing with no lease to answer, thrown on disposal. */
    readonly #failures: unknown[] = [];
    /** The durations of the waits for a compiler. */
    readonly #scope: TelemetryScope;
    /** The compilers in the pool, idle and used. */
    readonly #compilers;

    /** Store compilers running a toolchain into a store. */
    private constructor(store: LocalBuildStore, toolchain: () => Promise<Toolchain>) {
        // keep the store, the toolchain, the fingerprint and the scope
        this.store = store;
        this.#read = toolchain;
        this.fingerprint = CompilerFingerprint.of(store.environment);
        this.#scope = TelemetryScope.of(import.meta.destack.package, store.environment.telemetry);
        this.#compilers = this.#scope.metric.upDownCounter("destack.compiler.count", {
            unit: "{compiler}",
            attributes: { "destack.compiler.state": COMPILER_STATES },
        });
    }

    /** Count compilers entering or leaving a state. */
    #count(state: (typeof COMPILER_STATES)[number], delta: number): void {
        this.#compilers.add(delta, { "destack.compiler.state": state });
    }

    /** Open a pool starting the current compiler with a toolchain, building into a store. */
    static async open(
        store: LocalBuildStore,
        toolchain: () => Promise<Toolchain>,
    ): Promise<CompilerPool> {
        return new CompilerPool(store, toolchain);
    }

    /** Read the toolchain the compilers run, once it is first needed. */
    toolchain(): Promise<Toolchain> {
        return this.#toolchain.get(this.#read);
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
        const warm = await this.#scope.measure(
            "destack.compiler.wait_time",
            {},
            () =>
                new Promise<Warm>((take) => {
                    this.#waiting.push({ workspace, directory: resolved, take });
                    this.#serve();
                }),
        );

        // run the work, then free the compiler, closing one the pool released meanwhile
        const leased = Promise.withResolvers<void>();
        this.#leased.add(leased.promise);
        try {
            return await work(await warm.compiler);
        } finally {
            // count it idle again, or gone once the pool released it
            warm.isBusy = false;
            this.#count("used", -1);
            if (this.#warm.includes(warm)) {
                this.#count("idle", 1);
                this.#idle(warm);
            } else {
                void this.#dispose(warm);
            }
            this.#leased.delete(leased.promise);
            leased.resolve();
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
                this.#count("idle", -1);
                this.#count("used", 1);
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
        const isFull = this.#warm.length + this.#closing.size >= MAX_WARM_COMPILERS;

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
        const compiler = Compiler.start(workspace, {
            toolchain: () => this.toolchain(),
            store: this.store,
        });
        const idle = new AbortController();
        const warm: Warm = { compiler, workspace, directory, isBusy: false, use: this.#uses, idle };
        this.#warm.push(warm);
        this.#count("idle", 1);
        this.#idle(warm);

        // forget one that fails to start, which its lease answers, and serve the next waiter
        compiler.catch(() => {
            // stop its timer and count an idle one gone, a leased one counted once its lease ends
            warm.idle.abort();
            if (this.#warm.includes(warm) && !warm.isBusy) {
                this.#count("idle", -1);
            }

            // drop it and serve the next waiter
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
        // close the idle compilers, then wait for the running leases and every close they or idle timers started
        await this.#close(this.#warm);
        await Promise.all(this.#leased);
        await Promise.all(this.#closing);
        if (this.#failures.length > 0) {
            throw new AggregateError(this.#failures, "compilers did not close");
        }
    }

    /** Forget some warm compilers, closing each idle one now and each busy one after its lease. */
    async #close(closed: readonly Warm[]): Promise<void> {
        // stop their idle timers and drop them from the pool, a busy one counted until its lease ends
        for (const warm of closed) {
            warm.idle.abort();
            if (this.#warm.includes(warm) && !warm.isBusy) {
                this.#count("idle", -1);
            }
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

    /** Close a forgotten compiler, tracked until it stopped, then serve the waiters. */
    #dispose(warm: Warm): Promise<void> {
        const closing = this.#stop(warm).finally(() => {
            this.#closing.delete(closing);
            this.#serve();
        });
        this.#closing.add(closing);

        return closing;
    }

    /** Stop a compiler, recording a failure no lease answers. */
    async #stop(warm: Warm): Promise<void> {
        try {
            await warm.compiler.then(
                (started) => started[Symbol.asyncDispose](),
                () => undefined,
            );
        } catch (error) {
            this.#failures.push(error);
        }
    }
}
