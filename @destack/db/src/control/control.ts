import type { DatabaseConnection } from "../database/index.ts";
import { DatabaseError } from "../error/index.ts";
import { Lock, lock } from "../limit/index.ts";
import type { Tracker } from "../tracker/index.ts";
import { type Change, ReadSet, Snapshot } from "../log/index.ts";
import { TABLE, type Table } from "../table/index.ts";
import { found, nth, Uuid } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { Failure } from "@destack/error";
import { RetryPolicy, until, yieldToEventLoop, Clock } from "@destack/environment";
import type { Alarm } from "./alarm.ts";

/** The tracker topic a loop announces releasing its locks on. */
const RELEASED_TOPIC = "control/released";

/** The retry of a failed reconciliation, doubling from a second up to 5 minutes. */
const RETRY = RetryPolicy.of({ maximumInterval: 5 * 60_000 });

/** How long a key may fail transiently before its failures are reported: a minute. */
const TRANSIENT_REPORT_MILLISECONDS = 60_000;

/** The transient failures a key reaches before a report, its retries 1, 2 and 4 s apart. */
const TRANSIENT_REPORT_ATTEMPTS = 4;

/** A loop converging the actual state of each listed key to its rows. */
export interface ReconcileController {
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
    /** Converge each key and return, the default mode. */
    readonly mode?: "reconcile";
    /** Read a listed key's desired revision, whose change retries a failing key at once. */
    readonly revision?: (key: string) => string | undefined;
    /** Converge one key and return the delay before looking again, or follow it until stopped. */
    readonly reconcile: (
        key: string,
        reconciliation: Reconciliation,
    ) => Promise<number | undefined>;
}

/** A controller following each key until stopped, as an informer follows its source. */
export interface FollowController extends Omit<ReconcileController, "mode" | "keys" | "reconcile"> {
    /** Follow each key until stopped. */
    readonly mode: "follow";
    /** Follow one key until stopped, reporting once it caught up with its source. */
    readonly reconcile: (key: string, following: Following) => Promise<number | undefined>;
}

/** A controller a loop runs: converging keys, or following them. */
export type Controller = ReconcileController | FollowController;

/** The writes that make a follower list again: logged changes and written unlogged tables. */
export interface Delta {
    /** The logged changes of its watched tables. */
    readonly changes: readonly Change[];
    /** The unlogged watched tables commits wrote, by SQL name. */
    readonly unlogged: ReadonlySet<string>;
}

/** A key's backoff: its failures in a row, the last failure and the revision it failed at. */
interface Backoff {
    /** The failures in a row. */
    readonly count: number;
    /** The last failure. */
    readonly failure: Failure;
    /** Whether the loop reported the last failure, or an earlier one of the same reason. */
    readonly isReported: boolean;
    /** The revision the controller read for the key as it failed, absent without revisions. */
    readonly revision: string | undefined;
}

/** One running reconciliation of a key. */
export interface Reconciliation {
    /** Abort once the loop stops or the key leaves the list. */
    readonly signal: AbortSignal;
    /** Wait for the key to change again while it reconciles, as its controller's keys select it. */
    readonly changed: () => Promise<void>;
}

/** One running follow of a key, which reports once it caught up with its source. */
export interface Following extends Reconciliation {
    /** Report the follow caught up with its source, once per follow, which settling waits for. */
    readonly synced: () => void;
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
    /** The tables a loop keeps beside its controllers' on a shared database: their locks. */
    static readonly tables: readonly Table[] = [lock];

    /** The database with the log that selects keys. */
    readonly database: DatabaseConnection;
    /** The controllers run. */
    readonly controllers: readonly Controller[];
    /** Report a failed reconciliation. */
    readonly #report: (controller: Controller, key: string, error: unknown) => void;
    /** The loop's spans and captures, on the database's telemetry. */
    readonly #instruments: TelemetryScope;
    /** How a transiently failed key retries. */
    readonly #retry: RetryPolicy;
    /** The keys due, by controller and key, with the time each is due at. */
    readonly #due = new Map<Controller, Map<string, number>>();
    /** The due keys, earliest first. */
    readonly #queue = new DueQueue();
    /** The consecutive failures by controller and key. */
    readonly #failures = new Map<Controller, Map<string, Backoff>>();
    /** When each key first failed transiently in its current run of failures. */
    readonly #transient = new Map<Controller, Map<string, number>>();
    /** The keys reconciling now by controller with their abort controllers. */
    readonly #running = new Map<Controller, Map<string, AbortController>>();
    /** What each following controller's last list read. */
    readonly #reads = new Map<Controller, ReadSet>();
    /** Each controller's latest listing, which the next one waits for. */
    readonly #refreshing = new Map<Controller, Promise<void>>();
    /** The running reconciliations and follows. */
    readonly #reconciling = new Set<Promise<void>>();
    /** The running keys not settled yet, a follow settling once synced. */
    readonly #unsettled = new Set<symbol>();
    /** The tracker whose instance locks each key, when other connections share the database. */
    readonly #tracker?: Tracker;
    /** The lock of each controller, when other connections share the database. */
    readonly #locks = new Map<Controller, Lock>();
    /** Whether the loop leads each controller, settled once its lock is taken or refused. */
    readonly #leading = new Map<Controller, Promise<boolean>>();
    /** The keys waiting for another holder's lock, run again once it releases or stops. */
    readonly #waiting = new Map<Controller, Set<string>>();
    /** The host's wake-up for the earliest due key. */
    readonly #alarm?: Alarm;
    /** The clock keys come due at. */
    readonly #clock: Clock;
    /** How long a key may fail transiently before its failures are reported. */
    readonly #transientFor: number;
    /** The earliest due time the alarm is set to, absent for none. */
    #alarmed: number | undefined;
    /** The due time of the key that began the running reconciliations. */
    #busy: number | undefined;
    /** The callers waiting until no key is due now or unsettled, or the loop fails. */
    #idle: PromiseWithResolvers<void>[] = [];
    /** The abort of every loop once a controller fails undeclared, its reason the failure. */
    readonly #failing = new AbortController();
    /** Whether the log listens to other writers' commits, which idle waits for. */
    #isListening = false;
    /** Whether every controller was listed once since the loop started. */
    #isListed = false;
    /** The log sequence the follow consumed, moved forward by the follow and by idle checks. */
    #followed = 0;
    /** Aborts once an idle check found the follow behind a compaction, making it list again. */
    #relisting = new AbortController();
    /** The logged tables the controllers watch, which the follow reads. */
    readonly #watched: readonly Table[];
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
            /** Report a failed reconciliation, beside the loop recording it. */
            readonly report: (controller: Controller, key: string, error: unknown) => void;
            /** How a transiently failed key retries. */
            readonly retry?: Partial<RetryPolicy>;
            /** Set a wake-up for the earliest due key, so an evicted instance runs it. */
            readonly alarm?: Alarm;
            /** The clock keys come due at. */
            readonly clock: Clock;
            /** How long a key may fail transiently before reporting. */
            readonly transientFor?: number;
        },
    ) {
        // start with no key due and no failure
        this.database = database;
        this.controllers = controllers;
        const instruments = TelemetryScope.of(
            import.meta.destack.package,
            database.state.environment.telemetry,
        );
        this.#instruments = instruments;
        this.#report = (controller, key, error) => {
            // record the failure with the controller and the failed key, then report it
            instruments.captureException(error, {
                isEscaped: true,
                tags: { controller: controller.name },
                attributes: { "destack.key": key },
            });
            options.report(controller, key, error);
        };
        this.#retry = { ...RETRY, ...options.retry };
        this.#clock = options.clock;
        this.#transientFor = options.transientFor ?? TRANSIENT_REPORT_MILLISECONDS;
        if (options.alarm !== undefined) {
            this.#alarm = options.alarm;
        }

        // lock each controller as this loop when instances share the database
        if (database.isShared) {
            const { tracker } = database;
            const holder = Uuid.v4(database.state.environment.random);
            this.#tracker = tracker;
            for (const controller of controllers) {
                const name = ControlLoop.#lockName(controller);
                this.#locks.set(controller, new Lock(database, name, tracker.instance, holder));
            }
        }
        this.#watched = [
            ...new Set(controllers.flatMap((controller) => controller.watches ?? [])),
        ].filter((table) => table[TABLE].retention !== "none");
        for (const controller of controllers) {
            this.#due.set(controller, new Map());
            this.#failures.set(controller, new Map());
            this.#running.set(controller, new Map());
        }
    }

    /** Run the controllers until the signal aborts. */
    async run(signal: AbortSignal): Promise<void> {
        // stop every loop once one or a controller fails, the first failure kept
        const running = AbortSignal.any([signal, this.#failing.signal]);
        const stopping = (loop: Promise<void>) =>
            loop.catch((error: unknown) => {
                this.#failing.abort(error);
            });

        // run the waiting keys again once another holder releases its locks or stops
        const tracker = this.#tracker;
        const unwatch = [
            tracker?.watchStopped(() => this.#requeueWaiting()),
            tracker?.listen(RELEASED_TOPIC, () => this.#requeueWaiting()),
        ];
        try {
            // wait for every loop to end, its running reconciliations included
            await Promise.all(
                [
                    this.#listen(running),
                    this.#follow(running),
                    this.#listUnlogged(running),
                    this.#work(running),
                ].map(stopping),
            );

            // fail with the first failure that stopped the loops
            if (this.#failing.signal.aborted) {
                throw this.#failing.signal.reason;
            }
        }
        // release this instance's locks and settle the idle waiters
        finally {
            this.#isStopped = true;
            for (const stop of unwatch) {
                stop?.();
            }
            for (const waiter of this.#idle.splice(0)) {
                this.#settleIdle(waiter);
            }
            // leave a closing database's locks to its stopping instance, which frees them
            if (tracker !== undefined && !this.database.state.isClosing) {
                for (const [controller, leading] of this.#leading) {
                    if (await leading) {
                        await found(this.#locks, controller).release();
                    }
                }
                tracker.broadcast(RELEASED_TOPIC, null);
            }
        }
    }

    /** The keys waiting or settling: due now and not running, or running and not yet settled. */
    get depth(): number {
        // count each due key once, leaving the running keys their settling counts
        const now = this.#clock.now();
        let due = 0;
        for (const [controller, keys] of this.#due) {
            const running = found(this.#running, controller);
            for (const [key, at] of keys) {
                due += at <= now && !running.has(key) ? 1 : 0;
            }
        }

        return due + this.#unsettled.size;
    }

    /** The log sequence the loop consumed, behind the log's head while changes wait to be keyed. */
    get followed(): number {
        return this.#followed;
    }

    /** Settle once no key is due or reconciling, every follow synced and every change received. */
    idle(deadline?: number): Promise<void> {
        // settle at once when the loop stopped, failing with a controller's failure
        if (this.#isStopped) {
            return this.#failing.signal.aborted
                ? Promise.reject(this.#failing.signal.reason)
                : Promise.resolve();
        }

        // wait for the loop to report itself idle once woken
        const waiter = Promise.withResolvers<void>();
        const { promise, resolve } = waiter;
        this.#idle.push(waiter);
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

    /** Wait until the log listens to other writers' commits, failing as listening fails. */
    async #listen(signal: AbortSignal): Promise<void> {
        await Promise.race([this.database.log.listening(), until(signal)]);
        this.#isListening = !signal.aborted;
        this.#wake();
    }

    /** List every controller and follow the watched logged tables. */
    async #follow(signal: AbortSignal): Promise<void> {
        const tables = this.#watched;
        let listed = this.controllers;
        while (!signal.aborted) {
            // read the position before listing
            const after = (await this.database.log.position()).sequence;
            for (const controller of listed) {
                await this.#refresh(controller);
            }
            this.#followed = after;
            this.#isListed = true;
            this.#wake();
            if (tables.length === 0) {
                return;
            }

            // follow the changes after the listing until stopped or asked to list again
            this.#relisting = new AbortController();
            const following = AbortSignal.any([signal, this.#relisting.signal]);
            try {
                for await (const page of this.database.log.follow({ tables, after }, following)) {
                    for (const controller of this.controllers) {
                        // find the changes the controller watches
                        const watched = controller.watches ?? [];
                        const changes = page.changes.filter((change) =>
                            watched.includes(change.table),
                        );

                        // list a following controller again once a change touches its list's reads
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

                    // record the consumed sequence for idle waiters, and let other tasks run
                    this.#followed = Math.max(this.#followed, page.sequence);
                    this.#wake();
                    await this.#clock.trackHost(yieldToEventLoop());
                }

                // list the watching controllers again once an idle check asked
                listed = this.controllers.filter((controller) =>
                    (controller.watches ?? []).some((table) => tables.includes(table)),
                );
            } catch (error) {
                // list again after compaction the controllers following the tables
                if (!DatabaseError.is(error, "CHANGES_COMPACTED")) {
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

    /** List a controller's keys, recording what a follower's list read, which it must watch. */
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
                `${controller.name} lists through ${unwatched[TABLE].name} but does not watch it`,
            );
        }
        // refuse a follower watching tables its list never reads, which would never list again
        else if (watched.size > 0 && reads.tables.length === 0) {
            throw new TypeError(`${controller.name} watches tables, but its list reads no table`);
        }
        this.#reads.set(controller, reads);

        return keys;
    }

    /** List the keys a change touches. */
    async #keys(controller: ReconcileController, change: Change): Promise<readonly string[]> {
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

        // unqueue the due keys the list dropped, and forget their failures
        const due = found(this.#due, controller);
        const failures = found(this.#failures, controller);
        const transient = this.#transient.get(controller);
        for (const key of [...due.keys(), ...failures.keys()]) {
            if (!listed.has(key)) {
                due.delete(key);
                failures.delete(key);
                transient?.delete(key);
            }
        }

        // start the listed keys not running, a failing one only once its desired state changed
        for (const key of listed) {
            const failure = failures.get(key);
            const isRevised =
                failure !== undefined && failure.revision !== controller.revision?.(key);
            if (isRevised) {
                failures.delete(key);
            }
            if (!running.has(key) && (failure === undefined || isRevised)) {
                this.enqueue(controller, key);
            }
        }
    }

    /** Report whether the follow consumed every watched change, moving its cursor to the head. */
    async #isFollowed(): Promise<boolean> {
        // take a loop without logged tables as followed
        if (this.#watched.length === 0) {
            return true;
        }

        // read one watched change past the cursor, for the follow running as the read starts
        const relisting = this.#relisting;
        try {
            const page = await this.database.log.read({
                tables: this.#watched,
                after: this.#followed,
                limit: 1,
            });
            const isFollowed = page.changes.length === 0;
            if (isFollowed) {
                this.#followed = Math.max(this.#followed, page.sequence);
            }

            return isFollowed;
        } catch (error) {
            // ask that follow to list again behind a compaction, a later one having listed
            if (DatabaseError.is(error, "CHANGES_COMPACTED")) {
                relisting.abort();

                return false;
            }
            throw error;
        }
    }

    /** Start due keys and sleep until more are due. */
    async #work(signal: AbortSignal): Promise<void> {
        while (!signal.aborted) {
            // take the earliest runnable key
            this.#woken = new AbortController();
            const now = this.#clock.now();
            const { runnable, due, skipped } = this.#take(now);

            // start it, holding no lock or slot meanwhile
            if (runnable !== undefined) {
                this.#start(runnable);

                // set the wake-up due while the key runs
                await this.#schedule(this.#busy);

                // let other tasks run before the next key
                await this.#clock.trackHost(yieldToEventLoop());
            }
            // set the wake-up due while keys run, or at the next key
            else {
                await this.#schedule(this.#busy ?? due ?? skipped);

                // settle idle waiters once every key settled, none waits for a slot, and every change is followed
                if (
                    this.#idle.length > 0 &&
                    this.#isListed &&
                    this.#unsettled.size === 0 &&
                    skipped === undefined &&
                    this.#isListening &&
                    (await this.#isFollowed()) &&
                    !this.#woken.signal.aborted
                ) {
                    for (const waiter of this.#idle.splice(0)) {
                        waiter.resolve();
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

        // keep the key unsettled until it returns, or a follow until it syncs
        const unsettled = Symbol(work.key);
        this.#unsettled.add(unsettled);
        const settle = () => {
            if (this.#unsettled.delete(unsettled)) {
                this.#wake();
            }
        };
        const reconciling = this.#reconcile(work, stop.signal, settle).finally(() => {
            // free the key and its change waiters, settling it
            running.delete(work.key);
            this.#changes.get(work.controller)?.delete(work.key);
            settle();

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
    async #reconcile(work: Work, stopped: AbortSignal, synced: () => void): Promise<void> {
        // reconcile under the lock and requeue when asked
        const { controller, key } = work;
        const failures = found(this.#failures, controller);

        try {
            const delay = await this.#locked(work, stopped, async (reconciliation) => {
                // follow until stopped, reporting once caught up
                if (controller.mode === "follow") {
                    await controller.reconcile(key, { ...reconciliation, synced });
                    if (!reconciliation.signal.aborted) {
                        throw DatabaseError.of(
                            "FOLLOW_ENDED",
                            `${controller.name} stopped following ${key} without an abort`,
                        );
                    }

                    return stopped.aborted ? undefined : 0;
                }

                // trace each reconciliation as a separate root span
                const attributes = { "destack.controller": controller.name, "destack.key": key };

                return this.#instruments.span("controller.reconcile", attributes, () =>
                    controller.reconcile(key, reconciliation),
                );
            });
            failures.delete(work.key);
            this.#transient.get(work.controller)?.delete(work.key);
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

            // stop the loop on a failure the controller declares no outcome for
            if (!Failure.isDeclared(error)) {
                this.#failing.abort(
                    DatabaseError.of(
                        "CONTROLLER_FAILED",
                        `${work.controller.name} failed to reconcile ${work.key}`,
                        { cause: error },
                    ),
                );

                return;
            }

            // report a failure once, a transient one once it persists
            const previous = failures.get(work.key);
            const failure = Failure.of(error);
            const failed = (previous?.count ?? 0) + 1;
            const isRepeated =
                previous?.isReported === true &&
                previous.failure.domain === failure.domain &&
                previous.failure.reason === failure.reason;
            const isReported = !isRepeated && this.#recordFailure(work, failure, failed);
            if (isReported) {
                this.#report(work.controller, work.key, error);
            }
            failures.set(work.key, {
                count: failed,
                failure,
                isReported: isRepeated || isReported,
                revision: work.controller.revision?.(work.key),
            });

            // retry a transient failure on backoff, leaving any other to the key's next change
            if (Failure.isTransient(failure)) {
                const drawn = RetryPolicy.draw(`${work.controller.name} ${work.key} ${failed}`);
                this.enqueue(
                    work.controller,
                    work.key,
                    this.#clock.now() + RetryPolicy.interval(this.#retry, failed, () => drawn),
                );
            }
        }
    }

    /** Settle an idle waiter of the stopped loop, failing it with a controller's failure. */
    #settleIdle(waiter: PromiseWithResolvers<void>): void {
        if (this.#failing.signal.aborted) {
            waiter.reject(this.#failing.signal.reason);
        } else {
            waiter.resolve();
        }
    }

    /** Record when a key's transient failures began, answering whether one is due a report. */
    #recordFailure(work: Work, failure: Failure, attempts: number): boolean {
        // report a lasting failure at once
        const now = this.#clock.now();
        const since = this.#transient.get(work.controller) ?? new Map<string, number>();
        this.#transient.set(work.controller, since);
        if (!Failure.isTransient(failure)) {
            since.delete(work.key);

            return true;
        }

        // report a transient failure once it lasted past the window over several attempts
        const first = since.get(work.key) ?? now;
        since.set(work.key, first);

        return now - first >= this.#transientFor && attempts >= TRANSIENT_REPORT_ATTEMPTS;
    }

    /** Reconcile a key while the loop leads its controller. */
    async #locked(
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

        // reconcile without a lock
        const tracker = this.#tracker;
        if (tracker === undefined) {
            return reconcile({ signal: stopped, changed });
        }

        // mark the key waiting first, so a release while it takes the lock runs it again
        const { controller, key } = work;
        const waiting = this.#waiting.get(controller) ?? new Set();
        this.#waiting.set(controller, waiting.add(key));

        // lead the controller, its keys sharing one take of its lock
        let leading = this.#leading.get(controller);
        if (leading === undefined) {
            leading = found(this.#locks, controller).take((instance) => tracker.isLive(instance));
            this.#leading.set(controller, leading);
        }

        // wait for another holder's release, taking the lock afresh then
        if (!(await leading)) {
            if (this.#leading.get(controller) === leading) {
                this.#leading.delete(controller);
            }

            return undefined;
        }
        waiting.delete(key);

        return reconcile({ signal: stopped, changed });
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

    /** Run every waiting key again. */
    #requeueWaiting(): void {
        for (const [controller, keys] of this.#waiting) {
            for (const key of keys) {
                this.enqueue(controller, key);
            }
            keys.clear();
        }
    }

    /** Read the lock name of a controller. */
    static #lockName(controller: Controller): string {
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
