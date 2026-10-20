import {
    TABLE,
    type DatabaseConnection,
    type Table,
    DatabaseError,
    type Change,
    Lease,
    type LeaseOutcome,
    ReadSet,
    Snapshot,
} from "@destack/db";
import { nth, found } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { CallError } from "../error/error.ts";
import { RetryPolicy, until, yieldToEventLoop } from "../timer/index.ts";
import type { Alarm } from "./alarm.ts";
import { Clock } from "@destack/environment";

/** The default lease duration: 15 s, Kubernetes leader election's lease duration. */
export const LEASE_MILLISECONDS = 15_000;

/** The control loop's spans. */
const { span } = telemetry.scope(import.meta.destack.package);

/** The retry of a failed reconciliation, doubling from a second up to 5 minutes. */
const RETRY = RetryPolicy.of({ maximumInterval: 5 * 60_000 });

/** How long a key's peer may answer unavailable before its failures are reported: a minute. */
const UNAVAILABLE_REPORT_MILLISECONDS = 60_000;

/** A loop converging the actual state of each listed key to its rows. */
export interface Controller {
    /** The controller's name in reports. */
    readonly name: string;
    /** The tables with changes or commits that select keys or list them again. */
    readonly watches?: readonly Table[];
    /** List the keys a watched change affects, when reconciling. */
    readonly keys?: (change: Change) => readonly string[] | Promise<readonly string[]>;
    /** List every key through the view; a follower gets the changes that list it again. */
    readonly list: (view: Snapshot, delta?: Delta) => Promise<readonly string[]>;
    /** The most keys it works on at once, one by default. */
    readonly concurrency?: number;
    /** Whether a key converges and returns, or runs its process until stopped. */
    readonly mode?: "reconcile" | "follow";
    /** Converge one key and return the delay before looking again, or follow it until stopped. */
    readonly reconcile: (
        key: string,
        reconciliation: Reconciliation,
    ) => Promise<number | undefined>;
}

/** The writes that make a follower list again: logged changes and written unlogged tables. */
export interface Delta {
    /** The logged changes of its watched tables. */
    readonly changes: readonly Change[];
    /** The unlogged watched tables commits wrote, by SQL name. */
    readonly unlogged: ReadonlySet<string>;
}

/** One running reconciliation of a key. */
export interface Reconciliation {
    /** Abort once the loop stops, the lease is lost or the key leaves the list. */
    readonly signal: AbortSignal;
    /** The lease's takeover count, when instances share the loop. */
    readonly epoch?: number;
    /** Wait for the key to change again while it reconciles, as its controller's keys select it. */
    readonly changed: () => Promise<void>;
}

/** Reconciliations of keys whose work another host may end, such as by cancelling it. */
export const Reconciliation = {
    /** Run work with a signal that aborts once the reconciliation stops. */
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

        // run the work until it settles and stop checking
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
    /** When each key's peer first answered unavailable in its current run of failures. */
    readonly #unavailable = new Map<Controller, Map<string, number>>();
    /** The keys reconciling now by controller with their abort controllers. */
    readonly #running = new Map<Controller, Map<string, AbortController>>();
    /** What each following controller's last list read. */
    readonly #reads = new Map<Controller, ReadSet>();
    /** Each controller's latest listing, which the next one waits for. */
    readonly #refreshing = new Map<Controller, Promise<void>>();
    /** The running reconciliations. */
    readonly #reconciling = new Set<Promise<void>>();
    /** The lease holder and duration, when instances share the loop. */
    readonly #lease?: { readonly holder: string; readonly duration: number };
    /** The host's wake-up for the earliest due key. */
    readonly #alarm?: Alarm;
    /** The clock keys come due at. */
    readonly #clock: Clock;
    /** How long a key's peer may answer unavailable before its failures are reported. */
    readonly #unavailableFor: number;
    /** The earliest due time the alarm is set to, absent for none. */
    #alarmed: number | undefined;
    /** The due time of the key that began the running reconciliations. */
    #busy: number | undefined;
    /** The callers waiting until no key is due now or reconciling. */
    #idle: (() => void)[] = [];
    /** Whether every controller was listed once since the loop started. */
    #isListed = false;
    /** Whether the loop stopped, after which nothing more runs. */
    #isStopped = false;
    /** The waiters for a change of each reconciling key, by controller. */
    readonly #changes = new Map<Controller, Map<string, (() => void)[]>>();
    /** Aborts once work arrives after the worker last looked for it. */
    #woken = new AbortController();

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
            /** Set a wake-up for the earliest due key, so an evicted instance runs it. */
            readonly alarm?: Alarm;
            /** The clock keys come due at. */
            readonly clock: Clock;
            /** How long a key's peer may answer unavailable before reporting. */
            readonly unavailableFor?: number;
        },
    ) {
        // start with no key due and no failure
        this.database = database;
        this.controllers = controllers;
        this.#report = options.report;
        this.#retry = { ...RETRY, ...options.retry };
        this.#clock = options.clock;
        this.#unavailableFor = options.unavailableFor ?? UNAVAILABLE_REPORT_MILLISECONDS;
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
                [this.#follow(running), this.#listUnlogged(running), this.#work(running)].map(
                    stopping,
                ),
            );
        }
        // release this instance's leases and settle the idle waiters
        finally {
            this.#isStopped = true;
            for (const resolve of this.#idle.splice(0)) {
                resolve();
            }
            if (this.#lease !== undefined) {
                for (const controller of this.controllers) {
                    await Lease.release(this.database, {
                        holder: this.#lease.holder,
                        name: ControlLoop.#leaseName(controller),
                    });
                }
            }
        }
    }

    /** Settle once no key is due now or reconciling, or at a deadline. */
    idle(deadline?: number): Promise<void> {
        // settle at once when the loop stopped
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

        // settle at the deadline on the clock while late keys go on running and the alarm stays due
        const settled = new AbortController();
        this.#clock.sleep(deadline - this.#clock.now(), settled.signal).then(resolve, () => {});

        return promise.finally(() => settled.abort());
    }

    /** Mark a key due. */
    enqueue(controller: Controller, key: string, at = this.#clock.now()): void {
        // tell a reconciliation of the key that it changed
        for (const resolve of this.#changes.get(controller)?.get(key)?.splice(0) ?? []) {
            resolve();
        }

        // record the earliest due time
        const due = found(this.#due, controller);
        const current = due.get(key);
        if (current === undefined || at < current) {
            due.set(key, at);
            this.#queue.push({ controller, key, at });
        }
        this.#wake();
    }

    /** List every controller and follow the watched logged tables. */
    async #follow(signal: AbortSignal): Promise<void> {
        const tables = [
            ...new Set(this.controllers.flatMap((controller) => controller.watches ?? [])),
        ].filter((table) => table[TABLE].retention !== "none");
        let listed = this.controllers;
        while (!signal.aborted) {
            // read the position before listing
            const after = (await this.database.log.position()).sequence;
            for (const controller of listed) {
                await this.#refresh(controller);
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

                        // list a following controller again once a change touches what its list read
                        if (controller.mode === "follow") {
                            const reads = found(this.#reads, controller);
                            if (changes.some((change) => reads.touches(change))) {
                                await this.#refresh(controller, { changes, unlogged: new Set() });
                            }
                        }
                        // enqueue the keys each change touches
                        else {
                            for (const change of changes) {
                                for (const key of await this.#keys(controller, change)) {
                                    this.enqueue(controller, key);
                                }
                            }
                        }
                    }

                    // let other tasks run before the next page
                    await yieldToEventLoop();
                }
            } catch (error) {
                // list again after compaction the controllers following the tables
                if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                    throw error;
                }
                listed = this.controllers.filter((controller) =>
                    (controller.watches ?? []).some((table) => tables.includes(table)),
                );
            }
        }
    }

    /** List a controller's keys again: relist a follower, enqueue a reconciler's keys. */
    #refresh(controller: Controller, delta?: Delta): Promise<void> {
        // run one listing of a controller at a time, in the order asked
        const refreshing = (this.#refreshing.get(controller) ?? Promise.resolve()).then(() =>
            controller.mode === "follow"
                ? this.#relist(controller, delta)
                : this.#enqueueListed(controller),
        );
        this.#refreshing.set(
            controller,
            refreshing.catch(() => {}),
        );

        return refreshing;
    }

    /** Enqueue each key a reconciling controller lists, leaving failing ones to their backoff. */
    async #enqueueListed(controller: Controller): Promise<void> {
        const failures = found(this.#failures, controller);
        for (const key of await this.#list(controller)) {
            if (!failures.has(key)) {
                this.enqueue(controller, key);
            }
        }
    }

    /** List a controller's keys, keeping what a follower's list read, which it must watch. */
    async #list(controller: Controller, delta?: Delta): Promise<readonly string[]> {
        // list a reconciling controller through a plain live view
        if (controller.mode !== "follow") {
            return controller.list(Snapshot.live(this.database));
        }

        // record a follower's reads, refusing a read of a table it does not watch
        const reads = new ReadSet();
        const keys = await controller.list(Snapshot.live(this.database).recording(reads), delta);
        const watched = new Set(controller.watches ?? []);
        const unwatched = reads.tables.find((table) => !watched.has(table));
        if (unwatched !== undefined) {
            throw new TypeError(
                `${controller.name} lists through ${unwatched[TABLE].name}, which it does not watch`,
            );
        }
        // refuse a follower watching tables its list never reads, which would never list again
        else if (watched.size > 0 && reads.tables.length === 0) {
            throw new TypeError(
                `${controller.name} follows tables its list never reads through its view`,
            );
        }
        this.#reads.set(controller, reads);

        return keys;
    }

    /** List the keys a change touches. */
    async #keys(controller: Controller, change: Change): Promise<readonly string[]> {
        try {
            return (await controller.keys?.(change)) ?? [];
        } catch (error) {
            this.#report(controller, change.table[TABLE].name, error);

            return this.#list(controller);
        }
    }

    /** List the controllers of unlogged tables again after each commit to one of those tables. */
    async #listUnlogged(signal: AbortSignal): Promise<void> {
        // find each controller's unlogged tables
        const unlogged = new Map(
            this.controllers.map((controller) => [
                controller,
                (controller.watches ?? [])
                    .filter((table) => table[TABLE].retention === "none")
                    .map((table) => table[TABLE].sqlName),
            ]),
        );
        if (![...unlogged.values()].some((tables) => tables.length > 0)) {
            return;
        }
        await this.database.log.until(async (committed) => {
            // pick the controllers whose unlogged tables the commits wrote
            const listed = this.controllers.filter((controller) =>
                found(unlogged, controller).some((table) => committed.has(table)),
            );
            for (const controller of listed) {
                const tables = found(unlogged, controller).filter((table) => committed.has(table));
                await this.#refresh(controller, { changes: [], unlogged: new Set(tables) });
            }

            return false;
        }, signal);
    }

    /** Stop the keys a controller's list dropped, and enqueue the listed ones not running. */
    async #relist(controller: Controller, delta?: Delta): Promise<void> {
        // stop the running keys the list dropped
        const listed = new Set(await this.#list(controller, delta));
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
            this.#woken = new AbortController();
            const now = this.#clock.now();
            const { runnable, due, skipped } = this.#take(now);
            if (runnable !== undefined) {
                this.#start(runnable);

                // set the wake-up due while the key runs
                await this.#schedule(this.#busy);

                // let other tasks run before the next key
                await yieldToEventLoop();
            }
            // set the wake-up due while keys run, or at the next key
            else {
                await this.#schedule(this.#busy ?? due ?? skipped);

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

        // find the earliest skipped key
        const [earliest] = skipped;

        return earliest === undefined || taken.runnable !== undefined
            ? taken
            : { ...taken, skipped: earliest.at };
    }

    /** Start a reconciliation. */
    #start(work: Due): void {
        // mark the key running with the controller aborting it
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
        // reconcile under the lease and requeue when asked
        const { controller, key } = work;
        const failures = found(this.#failures, controller);
        try {
            const delay = await this.#leased(work, stopped, async (reconciliation) => {
                // follow until stopped
                if (controller.mode === "follow") {
                    await controller.reconcile(key, reconciliation);
                    if (!reconciliation.signal.aborted) {
                        throw new Error(
                            `${controller.name} stopped following ${key} without an abort`,
                        );
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
            this.#unavailable.get(work.controller)?.delete(work.key);
            if (delay !== undefined) {
                this.enqueue(work.controller, work.key, this.#clock.now() + delay);
            }
        }
        // end a reconciliation the loop stopped without a failure
        catch (error) {
            if (stopped.aborted) {
                failures.delete(work.key);

                return;
            }

            // report and retry the failure, a peer's unavailability once it persists
            const failed = (failures.get(work.key) ?? 0) + 1;
            failures.set(work.key, failed);
            if (this.#isReported(work, error)) {
                this.#report(work.controller, work.key, error);
            }
            this.enqueue(
                work.controller,
                work.key,
                this.#clock.now() + RetryPolicy.interval(this.#retry, failed),
            );
        }
    }

    /** Decide whether to report a failure at once. */
    #isReported(work: Work, error: unknown): boolean {
        // report any failure but a peer's unavailability at once
        const now = this.#clock.now();
        const since = this.#unavailable.get(work.controller) ?? new Map<string, number>();
        this.#unavailable.set(work.controller, since);
        if (!isUnavailable(error)) {
            since.delete(work.key);

            return true;
        }

        // report an unavailable peer once it stayed so past the window
        const first = since.get(work.key) ?? now;
        since.set(work.key, first);

        return now - first >= this.#unavailableFor;
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
            Lease.acquire(this.database, {
                name: ControlLoop.#leaseName(work.controller),
                key: work.key,
                holder,
                duration,
                now: this.#clock.now(),
            });
        const acquired = await acquire();
        if (acquired.kind === "busy") {
            return Math.max(0, acquired.lapsesAt - this.#clock.now());
        }

        // renew the lease at a third of its duration on the clock
        const lost = new AbortController();
        const reconciled = new AbortController();
        void this.#renew(acquire, work.key, duration / 3, lost, reconciled.signal);
        try {
            const signal = AbortSignal.any([stopped, lost.signal]);

            return await reconcile({ epoch: acquired.epoch, signal, changed });
        } finally {
            reconciled.abort();
        }
    }

    /** Renew an acquired lease each interval of the clock until reconciled, or abort it as lost. */
    async #renew(
        acquire: () => Promise<LeaseOutcome>,
        key: string,
        interval: number,
        lost: AbortController,
        reconciled: AbortSignal,
    ): Promise<void> {
        while (
            await this.#clock.sleep(interval, reconciled).then(
                () => true,
                () => false,
            )
        ) {
            // lose the lease once another holds it or renewing fails
            try {
                const renewed = await acquire();
                if (renewed.kind === "busy") {
                    lost.abort(new Error(`lease of ${key} lost`));

                    return;
                }
            } catch (error) {
                lost.abort(error);

                return;
            }
        }
    }

    /** Schedule the host's wake-up at the earliest due time, once per change. */
    async #schedule(due: number | undefined): Promise<void> {
        // leave an unchanged wake-up
        const alarm = this.#alarm;
        if (alarm === undefined || due === this.#alarmed) {
            return;
        }

        // move the wake-up or clear it once nothing is due
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

    /** Wake the worker for work that arrived, at once when it sleeps or after its current look. */
    #wake(): void {
        this.#woken.abort();
    }

    /** Sleep for a delay or until woken. */
    async #sleep(delay: number | undefined, signal: AbortSignal): Promise<void> {
        // wake at the clock's due time, or on work or stop
        const waking = AbortSignal.any([signal, this.#woken.signal]);
        try {
            await (delay === undefined ? until(waking) : this.#clock.sleep(delay, waking));
        } catch (error) {
            // end the sleep on a wake or a stop
            if (!waking.aborted) {
                throw error;
            }
        }
    }

    /** Read the lease name of a controller's keys. */
    static #leaseName(controller: Controller): string {
        return `controller:${controller.name}`;
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
            const above = nth(heap, parent);
            const below = nth(heap, index);
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
            if (left < heap.length && nth(heap, left).at < nth(heap, smallest).at) {
                smallest = left;
            }
            if (right < heap.length && nth(heap, right).at < nth(heap, smallest).at) {
                smallest = right;
            }
            if (smallest === index) {
                return earliest;
            }
            [heap[smallest], heap[index]] = [nth(heap, index), nth(heap, smallest)];
            index = smallest;
        }
    }
}

/** Report whether a failure is transiently unavailable, such as a peer down or a stale replica. */
function isUnavailable(error: unknown): boolean {
    // read the failure's status and code, a domain failure's through its call error
    const failure = CallError.of(error) ?? error;
    const status =
        typeof failure === "object" && failure !== null && "status" in failure
            ? failure.status
            : undefined;
    const code =
        typeof failure === "object" && failure !== null && "code" in failure
            ? failure.code
            : undefined;

    return (
        status === 503 ||
        code === "SERVICE_UNAVAILABLE" ||
        code === "ConnectionRefused" ||
        code === "ECONNREFUSED"
    );
}
