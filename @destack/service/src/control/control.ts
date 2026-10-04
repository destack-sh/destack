import {
    TABLE,
    type DatabaseConnection,
    type Table,
    DatabaseError,
    type Change,
} from "@destack/db";
import { aligned, found } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { RetryPolicy, wait } from "../timer/index.ts";
import type { Alarm } from "./alarm.ts";
import { LEASE_MILLISECONDS, Leases } from "./lease.ts";

/** The control loop's spans. */
const { span } = telemetry.scope(import.meta.destack.package);

/** The retry of a failed reconciliation, doubling from a second up to 5 minutes. */
const RETRY = RetryPolicy.of({ maximumInterval: 5 * 60_000 });

/** A loop keeping the actual state of each listed key matching its rows. */
export interface Controller {
    /** The controller's name in reports. */
    readonly name: string;
    /** The tables with changes or commits that select keys or list them again. */
    readonly watches?: readonly Table[];
    /** List the keys a watched change affects, when reconciling. */
    keys?(change: Change): readonly string[] | Promise<readonly string[]>;
    /** List every key. */
    list(): Promise<readonly string[]>;
    /** The most keys it works on at once, one by default. */
    readonly concurrency?: number;
    /** Whether a key converges and returns, or keeps its process running until stopped, reconcile by default. */
    readonly mode?: "reconcile" | "follow";
    /** Converge one key and return the delay before looking again, or follow it until stopped. */
    reconcile(key: string, reconciliation: Reconciliation): Promise<number | undefined>;
}

/** One running reconciliation of a key. */
export interface Reconciliation {
    /** Abort once the loop stops, the lease is lost or the key leaves the list. */
    readonly signal: AbortSignal;
    /** The lease's takeover count, when instances share the loop. */
    readonly epoch?: number;
    /** Wait for the key to change again while it reconciles, as its controller's keys select it. */
    changed(): Promise<void>;
}

/** Reconciliations of keys whose work another host may end, such as by cancelling it. */
export const Reconciliation = {
    /** Run work with a signal that aborts once the reconciliation stops or a check at a change of the key returns why the work ended elsewhere. */
    async runUntil<Result>(
        reconciliation: Reconciliation,
        stop: () => Promise<Error | undefined>,
        work: (signal: AbortSignal) => Promise<Result>,
    ): Promise<Result> {
        // check again at each change of the key until the work settles
        const ended = new AbortController();
        const settled = new AbortController();
        const settling = new Promise<void>((resolve) => {
            settled.signal.addEventListener("abort", () => resolve(), { once: true });
        });
        const watching = (async () => {
            while (!settled.signal.aborted) {
                await Promise.race([reconciliation.changed(), settling]);
                const reason = settled.signal.aborted ? undefined : await stop();
                if (reason !== undefined) {
                    ended.abort(reason);

                    return;
                }
            }
        })();

        // run the work until it settles, then stop checking
        try {
            return await work(AbortSignal.any([ended.signal, reconciliation.signal]));
        } finally {
            settled.abort();
            await watching;
        }
    },
};

/** One key a controller reconciles, due at a time. */
interface Work {
    /** The controller reconciling the key. */
    readonly controller: Controller;
    /** The key. */
    readonly key: string;
}

/** Run controllers over one database. */
export class ControlLoop {
    /** The database with the log that selects keys. */
    readonly database: DatabaseConnection;
    /** The controllers run. */
    readonly controllers: readonly Controller[];
    /** Report a failed reconciliation. */
    readonly #report: (controller: Controller, key: string, error: unknown) => void;
    /** How a failed key retries. */
    readonly #retry: RetryPolicy;
    /** The keys due, by controller and key, with the time each is due at. */
    readonly #due = new Map<Controller, Map<string, number>>();
    /** The due keys, earliest first. */
    readonly #queue = new DueQueue();
    /** The consecutive failures by controller and key. */
    readonly #failures = new Map<Controller, Map<string, number>>();
    /** The keys reconciling now by controller with their abort controllers. */
    readonly #running = new Map<Controller, Map<string, AbortController>>();
    /** The running reconciliations. */
    readonly #reconciling = new Set<Promise<void>>();
    /** The lease holder and duration, when instances share the loop. */
    readonly #lease?: { readonly holder: string; readonly duration: number };
    /** The wake-up the host keeps for the earliest due key. */
    readonly #alarm?: Alarm;
    /** The earliest due time the alarm keeps, absent for none. */
    #alarmed: number | undefined;
    /** The due time of the key that began the running reconciliations, where the alarm stays while any runs. */
    #busy: number | undefined;
    /** The callers waiting until no key is due now or reconciling. */
    #idle: (() => void)[] = [];
    /** Whether every controller was listed once since the loop started. */
    #isListed = false;
    /** Whether the loop stopped, after which nothing more runs. */
    #isStopped = false;
    /** The waiters for a change of each reconciling key, by controller. */
    readonly #changes = new Map<Controller, Map<string, (() => void)[]>>();
    /** Wake the worker waiting for due work. */
    #wake: () => void = () => {};

    /** Create the loop. */
    constructor(
        database: DatabaseConnection,
        controllers: readonly Controller[],
        options: {
            /** Report a failed reconciliation. */
            readonly report: (controller: Controller, key: string, error: unknown) => void;
            /** How a failed key retries. */
            readonly retry?: Partial<RetryPolicy>;
            /** Lease each key to this instance. */
            readonly lease?: { readonly holder: string; readonly duration?: number };
            /** Keep a wake-up for the earliest due key, so an evicted instance runs it. */
            readonly alarm?: Alarm;
        },
    ) {
        // start with no key due and no failure
        this.database = database;
        this.controllers = controllers;
        this.#report = options.report;
        this.#retry = { ...RETRY, ...options.retry };
        if (options.alarm !== undefined) {
            this.#alarm = options.alarm;
        }
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
        // stop every loop once one fails
        const failed = new AbortController();
        const running = AbortSignal.any([signal, failed.signal]);
        const stopping = (part: Promise<void>) =>
            part.catch((error: unknown) => {
                failed.abort(error);
                throw error;
            });
        try {
            await Promise.all(
                [this.#follow(running), this.#list(running), this.#work(running)].map(stopping),
            );
        }
        // release this instance's leases, and settle the idle waiters of a loop that runs nothing more
        finally {
            this.#isStopped = true;
            for (const resolve of this.#idle.splice(0)) {
                resolve();
            }
            if (this.#lease !== undefined) {
                await Leases.release(this.database, this.#lease.holder);
            }
        }
    }

    /** Settle once no key is due now or reconciling, or at a deadline, as an alarm's handler waits before it returns. */
    idle(deadline?: number): Promise<void> {
        // settle at once once the loop stopped
        if (this.#isStopped) {
            return Promise.resolve();
        }

        // wait for the loop to report itself idle once woken
        const { promise, resolve } = Promise.withResolvers<void>();
        this.#idle.push(resolve);
        this.#wake();
        if (deadline === undefined) {
            return promise;
        }

        // settle at the deadline at the latest while late keys keep running and the alarm stays due
        const timer = setTimeout(resolve, Math.max(0, deadline - Date.now()));

        return promise.finally(() => clearTimeout(timer));
    }

    /** Name a key due. */
    enqueue(controller: Controller, key: string, at = Date.now()): void {
        // tell a reconciliation of the key that it changed
        for (const resolve of this.#changes.get(controller)?.get(key)?.splice(0) ?? []) {
            resolve();
        }

        // keep the earliest due time
        const due = found(this.#due, controller);
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
        ].filter((table) => table[TABLE].retention !== "none");
        while (!signal.aborted) {
            // read the position before listing
            const after = (await this.database.log.position()).sequence;
            for (const controller of this.controllers) {
                for (const key of await controller.list()) {
                    this.enqueue(controller, key);
                }
            }
            this.#isListed = true;
            this.#wake();
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

                        // list a following controller again once its tables changed
                        if (controller.mode === "follow") {
                            if (changes.length > 0) {
                                await this.#relist(controller);
                            }
                        }
                        // enqueue the keys each change names
                        else {
                            for (const change of changes) {
                                for (const key of (await controller.keys?.(change)) ?? []) {
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
            (controller.watches ?? []).some((table) => table[TABLE].retention === "none"),
        );
        if (listed.length === 0) {
            return;
        }
        await this.database.log.until(async () => {
            for (const controller of listed) {
                // stop and start a following controller's keys
                if (controller.mode === "follow") {
                    await this.#relist(controller);
                }
                // enqueue each listed key not failing
                else {
                    const failures = found(this.#failures, controller);
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

    /** Stop the keys a controller's list dropped, and enqueue the listed ones not running. */
    async #relist(controller: Controller): Promise<void> {
        // stop the running keys the list dropped
        const listed = new Set(await controller.list());
        const running = found(this.#running, controller);
        for (const [key, stop] of running) {
            if (!listed.has(key)) {
                stop.abort(new Error(`${controller.name} no longer lists ${key}`));
            }
        }

        // unqueue the due keys the list dropped
        const due = found(this.#due, controller);
        for (const key of due.keys()) {
            if (!listed.has(key)) {
                due.delete(key);
            }
        }

        // start the listed keys neither running nor failing
        const failures = found(this.#failures, controller);
        for (const key of listed) {
            if (!running.has(key) && !failures.has(key)) {
                this.enqueue(controller, key);
            }
        }
    }

    /** Start due keys and sleep until more are due. */
    async #work(signal: AbortSignal): Promise<void> {
        while (!signal.aborted) {
            // start the earliest runnable key
            const now = Date.now();
            const { runnable, due, skipped } = this.#take(now);
            if (runnable !== undefined) {
                this.#start(runnable);
            }
            // keep the wake-up due while keys run or at the next key
            else {
                await this.#keep(this.#busy ?? due ?? skipped);

                // settle idle waiters once nothing reconciles
                if (this.#isListed && this.#reconciling.size === 0) {
                    for (const resolve of this.#idle.splice(0)) {
                        resolve();
                    }
                }

                // sleep until the next key is due
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
    #take(now: number): {
        readonly runnable?: Due;
        readonly due?: number;
        readonly skipped?: number;
    } {
        // skip running keys and keys of a full controller
        const skipped: Due[] = [];
        let taken: { readonly runnable?: Due; readonly due?: number; readonly skipped?: number } =
            {};
        for (let next = this.#next(); next !== undefined; next = this.#next()) {
            const running = found(this.#running, next.controller);
            const isFull = running.size >= (next.controller.concurrency ?? 1);
            if (next.at > now) {
                taken = { due: next.at };
                break;
            } else if (isFull || running.has(next.key)) {
                this.#queue.pop();
                skipped.push(next);
            } else {
                this.#queue.pop();
                found(this.#due, next.controller).delete(next.key);
                taken = { runnable: next };
                break;
            }
        }

        // queue the skipped keys again
        for (const entry of skipped) {
            this.#queue.push(entry);
        }

        // name the earliest skipped key, which the host's wake-up keeps while it waits
        const [earliest] = skipped;

        return earliest === undefined || taken.runnable !== undefined
            ? taken
            : { ...taken, skipped: earliest.at };
    }

    /** Start a reconciliation. */
    #start(work: Due): void {
        // mark the key running with the controller aborting it, the first running key holding the wake-up at its due time
        const running = found(this.#running, work.controller);
        const stop = new AbortController();
        running.set(work.key, stop);
        this.#busy ??= work.at;
        const reconciling = this.#reconcile(work, stop.signal).finally(() => {
            // free the key and its change waiters
            running.delete(work.key);
            this.#changes.get(work.controller)?.delete(work.key);

            // release the wake-up after the last reconciliation
            this.#reconciling.delete(reconciling);
            if (this.#reconciling.size === 0) {
                this.#busy = undefined;
            }

            // look for more work
            this.#wake();
        });
        this.#reconciling.add(reconciling);
    }

    /** Reconcile one key until settled, or follow it until stopped, retrying failures. */
    async #reconcile(work: Work, stopped: AbortSignal): Promise<void> {
        // reconcile under the lease and requeue when asked, or follow again once a follow ends
        const { controller, key } = work;
        const failures = found(this.#failures, controller);
        try {
            const delay = await this.#leased(work, stopped, async (reconciliation) => {
                // follow until stopped, retrying a lost lease at once and a follow that ended as a failure
                if (controller.mode === "follow") {
                    await controller.reconcile(key, reconciliation);
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
        // end a reconciliation the loop stopped without a failure, forgetting earlier ones
        catch (error) {
            if (stopped.aborted) {
                failures.delete(work.key);

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
        // wait for the key's changes while it reconciles
        const changed = () => {
            // add a waiter the key's next change resolves
            const waiting = this.#changes.get(work.controller) ?? new Map<string, (() => void)[]>();
            this.#changes.set(work.controller, waiting);
            const { promise, resolve } = Promise.withResolvers<void>();
            waiting.set(work.key, [...(waiting.get(work.key) ?? []), resolve]);

            return promise;
        };

        // reconcile without a lease
        const options = this.#lease;
        if (options === undefined) {
            return reconcile({ signal: stopped, changed });
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

            return await reconcile({ epoch: acquired.epoch, signal, changed });
        } finally {
            clearInterval(renewing);
        }
    }

    /** Keep the host's wake-up at the earliest due time, once per change. */
    async #keep(due: number | undefined): Promise<void> {
        // leave an unchanged wake-up
        const alarm = this.#alarm;
        if (alarm === undefined || due === this.#alarmed) {
            return;
        }

        // move the wake-up, or clear it once nothing is due
        this.#alarmed = due;
        await (due === undefined ? alarm.deleteAlarm() : alarm.setAlarm(due));
    }

    /** Find the earliest due key. */
    #next(): Due | undefined {
        for (let next = this.#queue.peek(); next !== undefined; next = this.#queue.peek()) {
            if (found(this.#due, next.controller).get(next.key) === next.at) {
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
            const above = aligned(heap, parent);
            const below = aligned(heap, index);
            if (above.at <= below.at) {
                break;
            }
            [heap[parent], heap[index]] = [below, above];
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
            if (left < heap.length && aligned(heap, left).at < aligned(heap, smallest).at) {
                smallest = left;
            }
            if (right < heap.length && aligned(heap, right).at < aligned(heap, smallest).at) {
                smallest = right;
            }
            if (smallest === index) {
                return earliest;
            }
            [heap[smallest], heap[index]] = [aligned(heap, index), aligned(heap, smallest)];
            index = smallest;
        }
    }
}
