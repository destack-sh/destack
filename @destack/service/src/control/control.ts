import { TABLE, type DatabaseConnection, type Table } from "@destack/db";
import { DatabaseError } from "@destack/db/error";
import type { Change } from "@destack/db/log";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { RetryPolicy, wait } from "../timer/index.ts";
import { LEASE_MILLISECONDS, Leases } from "./lease.ts";

/** The control loop's spans. */
const { span } = telemetry.scope(import.meta.destack.package);

/** The retry of a failed reconciliation, doubling from a second up to 5 minutes as in controller-runtime. */
const RETRY = RetryPolicy.of({ maximumInterval: 5 * 60_000 });

/** The keys a controller or follower works on: the tables naming them, and every key. */
interface Keyed {
    /** The controller's name in reports. */
    readonly name: string;
    /** The tables whose changes or commits name keys, or list them again. */
    readonly watches?: readonly Table[];
    /** Name the keys a watched change affects. */
    keys?(change: Change): readonly string[];
    /** List every key. */
    list(): Promise<readonly string[]>;
    /** The most keys it works on at once, one by default. */
    readonly concurrency?: number;
}

/** A level-triggered controller that reconciles each key until settled. */
export interface Controller extends Keyed {
    /** Reconcile one key, returning the delay before looking again. */
    reconcile(key: string, reconciliation: Reconciliation): Promise<number | undefined>;
}

/** A follower of each key, for as long as its list names the key and its lease holds. */
export interface Follower extends Keyed {
    /** Follow one key until the signal aborts. */
    follow(key: string, signal: AbortSignal): Promise<void>;
}

/** One running reconciliation of a key. */
export interface Reconciliation {
    /** Abort once the loop stops, the lease is lost or the key leaves the list. */
    readonly signal: AbortSignal;
    /** The lease's takeover count, when instances share the loop. */
    readonly epoch?: number;
}

/** One key a controller reconciles, due at a time. */
interface Work {
    /** The controller reconciling the key. */
    readonly controller: Controller | Follower;
    /** The key. */
    readonly key: string;
}

/** Run controllers over one database. */
export class ControlLoop {
    /** The database whose log names the keys. */
    readonly database: DatabaseConnection;
    /** The controllers run. */
    readonly controllers: readonly (Controller | Follower)[];
    /** Report a failed reconciliation. */
    readonly #report: (controller: Controller | Follower, key: string, error: unknown) => void;
    /** How a failed key retries. */
    readonly #retry: RetryPolicy;
    /** The keys due, by controller and key, with the time each is due at. */
    readonly #due = new Map<Controller | Follower, Map<string, number>>();
    /** The due keys, earliest first. */
    readonly #queue = new DueQueue();
    /** The consecutive failures by controller and key. */
    readonly #failures = new Map<Controller | Follower, Map<string, number>>();
    /** The keys reconciling now, by controller, each with the controller aborting it. */
    readonly #running = new Map<Controller | Follower, Map<string, AbortController>>();
    /** The running reconciliations. */
    readonly #reconciling = new Set<Promise<void>>();
    /** The lease holder and duration, when instances share the loop. */
    readonly #lease?: { readonly holder: string; readonly duration: number };
    /** Wake the worker waiting for due work. */
    #wake: () => void = () => {};

    /** Create the loop. */
    constructor(
        database: DatabaseConnection,
        controllers: readonly (Controller | Follower)[],
        options: {
            /** Report a failed reconciliation. */
            readonly report: (
                controller: Controller | Follower,
                key: string,
                error: unknown,
            ) => void;
            /** How a failed key retries. */
            readonly retry?: Partial<RetryPolicy>;
            /** Lease each key to this instance. */
            readonly lease?: { readonly holder: string; readonly duration?: number };
        },
    ) {
        // start with no key due and no failure
        this.database = database;
        this.controllers = controllers;
        this.#report = options.report;
        this.#retry = { ...RETRY, ...options.retry };
        if (options.lease !== undefined) {
            this.#lease = {
                holder: options.lease.holder,
                duration: options.lease.duration ?? LEASE_MILLISECONDS,
            };
        }
        for (const controller of controllers) {
            this.#due.set(controller, new Map());
            this.#failures.set(controller, new Map());
            this.#running.set(controller, new Map());
        }
    }

    /** Run the controllers until the signal aborts. */
    async run(signal: AbortSignal): Promise<void> {
        await Promise.all([this.#follow(signal), this.#list(signal), this.#work(signal)]);

        // release this instance's leases
        if (this.#lease !== undefined) {
            await Leases.release(this.database, this.#lease.holder);
        }
    }

    /** Name a key due. */
    enqueue(controller: Controller | Follower, key: string, at = Date.now()): void {
        // keep the earliest due time
        const due = this.#due.get(controller)!;
        const current = due.get(key);
        if (current === undefined || at < current) {
            due.set(key, at);
            this.#queue.push({ controller, key, at });
        }
        this.#wake();
    }

    /** List every controller, then follow the watched logged tables. */
    async #follow(signal: AbortSignal): Promise<void> {
        const tables = [
            ...new Set(this.controllers.flatMap((controller) => controller.watches ?? [])),
        ].filter((table) => table[TABLE].tier !== "none");
        while (!signal.aborted) {
            // read the position before listing
            const after = (await this.database.log.position()).sequence;
            for (const controller of this.controllers) {
                for (const key of await controller.list()) {
                    this.enqueue(controller, key);
                }
            }
            if (tables.length === 0) {
                return;
            }

            // follow the changes after the listing
            try {
                for await (const page of this.database.log.follow({ tables, after }, signal)) {
                    for (const controller of this.controllers) {
                        // find the changes the controller watches
                        const watched = controller.watches ?? [];
                        const changes = page.changes.filter((change) =>
                            watched.includes(change.table),
                        );

                        // list a follower again once its tables changed
                        if ("follow" in controller) {
                            if (changes.length > 0) {
                                await this.#relist(controller);
                            }
                        }
                        // enqueue the keys each change names
                        else {
                            for (const change of changes) {
                                for (const key of controller.keys?.(change) ?? []) {
                                    this.enqueue(controller, key);
                                }
                            }
                        }
                    }
                }
            } catch (error) {
                // list again after compaction
                if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                    throw error;
                }
            }
        }
    }

    /** List the controllers of unlogged tables after each commit. */
    async #list(signal: AbortSignal): Promise<void> {
        const listed = this.controllers.filter((controller) =>
            (controller.watches ?? []).some((table) => table[TABLE].tier === "none"),
        );
        if (listed.length === 0) {
            return;
        }
        await this.database.log.until(async () => {
            for (const controller of listed) {
                // stop and start a follower's keys
                if ("follow" in controller) {
                    await this.#relist(controller);
                }
                // enqueue each listed key not failing
                else {
                    const failures = this.#failures.get(controller)!;
                    for (const key of await controller.list()) {
                        if (!failures.has(key)) {
                            this.enqueue(controller, key);
                        }
                    }
                }
            }

            return false;
        }, signal);
    }

    /** Stop the keys a follower's list dropped, and enqueue the listed ones not running. */
    async #relist(controller: Follower): Promise<void> {
        // stop the running keys the list dropped
        const listed = new Set(await controller.list());
        const running = this.#running.get(controller)!;
        for (const [key, stop] of running) {
            if (!listed.has(key)) {
                stop.abort(new Error(`${controller.name} no longer lists ${key}`));
            }
        }

        // unqueue the due keys the list dropped
        const due = this.#due.get(controller)!;
        for (const key of due.keys()) {
            if (!listed.has(key)) {
                due.delete(key);
            }
        }

        // start the listed keys not running
        for (const key of listed) {
            if (!running.has(key)) {
                this.enqueue(controller, key);
            }
        }
    }

    /** Start due keys and sleep until more are due. */
    async #work(signal: AbortSignal): Promise<void> {
        while (!signal.aborted) {
            // start the earliest runnable key
            const now = Date.now();
            const { runnable, due } = this.#take(now);
            if (runnable !== undefined) {
                this.#start(runnable);
            }
            // sleep until due or woken
            else {
                await this.#sleep(due === undefined ? undefined : Math.max(0, due - now), signal);
            }
        }

        // stop and wait for running reconciliations
        for (const running of this.#running.values()) {
            for (const stop of running.values()) {
                stop.abort(new Error("the control loop stopped"));
            }
        }
        await Promise.all(this.#reconciling);
    }

    /** Take the earliest runnable key, or the next due time. */
    #take(now: number): { readonly runnable?: Due; readonly due?: number } {
        // skip keys whose controller is full or which run now
        const held: Due[] = [];
        let found: { readonly runnable?: Due; readonly due?: number } = {};
        for (let next = this.#next(); next !== undefined; next = this.#next()) {
            const running = this.#running.get(next.controller)!;
            const isFull = running.size >= (next.controller.concurrency ?? 1);
            if (next.at > now) {
                found = { due: next.at };
                break;
            } else if (isFull || running.has(next.key)) {
                held.push(this.#queue.pop()!);
            } else {
                this.#queue.pop();
                this.#due.get(next.controller)!.delete(next.key);
                found = { runnable: next };
                break;
            }
        }

        // queue the skipped keys again
        for (const entry of held) {
            this.#queue.push(entry);
        }

        return found;
    }

    /** Start a reconciliation. */
    #start(work: Work): void {
        // mark the key running with the controller aborting it
        const running = this.#running.get(work.controller)!;
        const stop = new AbortController();
        running.set(work.key, stop);
        const reconciling = this.#reconcile(work, stop.signal).finally(() => {
            running.delete(work.key);
            this.#reconciling.delete(reconciling);
            this.#wake();
        });
        this.#reconciling.add(reconciling);
    }

    /** Reconcile or follow one key until the loop stops it, retrying failures. */
    async #reconcile(work: Work, stopped: AbortSignal): Promise<void> {
        // reconcile under the lease and requeue when asked, or follow again once a follow ends
        const { controller, key } = work;
        const failures = this.#failures.get(controller)!;
        try {
            const delay = await this.#leased(work, stopped, async (reconciliation) => {
                // follow until stopped, retrying a lost lease at once and a follow that ended as a failure
                if ("follow" in controller) {
                    await controller.follow(key, reconciliation.signal);
                    if (!reconciliation.signal.aborted) {
                        throw new Error(`${controller.name} stopped following ${key} unasked`);
                    }

                    return stopped.aborted ? undefined : 0;
                }

                // trace each reconciliation as its own root span
                const attributes = { "destack.controller": controller.name, "destack.key": key };

                return span("controller.reconcile", attributes, () =>
                    controller.reconcile(key, reconciliation),
                );
            });
            failures.delete(work.key);
            if (delay !== undefined) {
                this.enqueue(work.controller, work.key, Date.now() + delay);
            }
        }
        // end a reconciliation the loop stopped without a failure
        catch (error) {
            if (stopped.aborted) {
                return;
            }

            // report and retry the failure
            const failed = (failures.get(work.key) ?? 0) + 1;
            failures.set(work.key, failed);
            this.#report(work.controller, work.key, error);
            this.enqueue(
                work.controller,
                work.key,
                Date.now() + RetryPolicy.interval(this.#retry, failed),
            );
        }
    }

    /** Reconcile a key under this instance's lease, stopped by the loop or a lost lease. */
    async #leased(
        work: Work,
        stopped: AbortSignal,
        reconcile: (reconciliation: Reconciliation) => Promise<number | undefined>,
    ): Promise<number | undefined> {
        // reconcile without a lease
        const options = this.#lease;
        if (options === undefined) {
            return reconcile({ signal: stopped });
        }

        // take the lease or wait for it to lapse
        const { holder, duration } = options;
        const acquire = () =>
            Leases.acquire(this.database, work.controller.name, work.key, holder, duration);
        const acquired = await acquire();
        if ("lapsesAt" in acquired) {
            return Math.max(0, acquired.lapsesAt - Date.now());
        }

        // renew the lease at a third of its duration
        const lost = new AbortController();
        const renewing = setInterval(() => {
            acquire().then(
                (renewed) => {
                    if ("lapsesAt" in renewed) {
                        lost.abort(new Error(`lease of ${work.key} lost`));
                    }
                },
                (error: unknown) => lost.abort(error),
            );
        }, duration / 3);
        try {
            const signal = AbortSignal.any([stopped, lost.signal]);

            return await reconcile({ epoch: acquired.epoch, signal });
        } finally {
            clearInterval(renewing);
        }
    }

    /** Find the earliest due key. */
    #next(): Due | undefined {
        for (let next = this.#queue.peek(); next !== undefined; next = this.#queue.peek()) {
            if (this.#due.get(next.controller)!.get(next.key) === next.at) {
                return next;
            }
            this.#queue.pop();
        }

        return undefined;
    }

    /** Sleep for a delay or until woken. */
    async #sleep(delay: number | undefined, signal: AbortSignal): Promise<void> {
        // wake at the due time, on work or on stop
        const woken = new AbortController();
        this.#wake = () => woken.abort();
        const until = AbortSignal.any([signal, woken.signal]);
        try {
            await wait(delay ?? Number.POSITIVE_INFINITY, { signal: until });
        } catch (error) {
            // end the sleep on a wake or a stop
            if (!until.aborted) {
                throw error;
            }
        } finally {
            this.#wake = () => {};
        }
    }
}

/** One key due at a time. */
interface Due extends Work {
    /** The time the key is due at, in UTC epoch milliseconds. */
    readonly at: number;
}

/** Due keys in a binary min-heap by due time. */
class DueQueue {
    /** The heap. */
    readonly #heap: Due[] = [];

    /** Read the earliest entry. */
    peek(): Due | undefined {
        return this.#heap[0];
    }

    /** Add an entry. */
    push(entry: Due): void {
        // sift the entry up
        const heap = this.#heap;
        heap.push(entry);
        let index = heap.length - 1;
        while (index > 0) {
            const parent = (index - 1) >> 1;
            if (heap[parent]!.at <= heap[index]!.at) {
                break;
            }
            [heap[parent], heap[index]] = [heap[index]!, heap[parent]!];
            index = parent;
        }
    }

    /** Remove the earliest entry. */
    pop(): Due | undefined {
        // move the last entry to the root
        const heap = this.#heap;
        const earliest = heap[0];
        const last = heap.pop();
        if (heap.length === 0 || last === undefined) {
            return earliest;
        }
        heap[0] = last;

        // sift it down
        let index = 0;
        while (true) {
            const left = 2 * index + 1;
            const right = left + 1;
            let smallest = index;
            if (left < heap.length && heap[left]!.at < heap[smallest]!.at) {
                smallest = left;
            }
            if (right < heap.length && heap[right]!.at < heap[smallest]!.at) {
                smallest = right;
            }
            if (smallest === index) {
                return earliest;
            }
            [heap[smallest], heap[index]] = [heap[index]!, heap[smallest]!];
            index = smallest;
        }
    }
}
