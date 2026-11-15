import type { DatabaseConnection } from "../database/index.ts";
import { DatabaseError } from "../error/index.ts";
import { Lock, lock } from "../limit/index.ts";
import type { Tracker } from "../tracker/index.ts";
import { Change, LogColumn, type LogCursor } from "../log/index.ts";
import { LOG, LOG_CHECKPOINT } from "../log/schema.ts";
import { and, eq, gt, gte, lte, min, or, Placeholder, type SQL, sql } from "../sql/index.ts";
import {
    Condition,
    type Match,
    ModelQueryBuilder,
    Namespace,
    Predicate,
    type Relation,
    Scalar,
} from "../query/index.ts";
import {
    type ColumnValue,
    TABLE,
    type Row,
    type RowImage,
    type Select,
    type Table,
} from "../table/index.ts";
import { canonicalize, Duration, found, nth, present, schema, Uuid } from "@destack/schema";
import { type Link, TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { Failure } from "@destack/error";
import { RetryPolicy, waitAborted, Clock } from "@destack/environment";
import type { Alarm } from "./alarm.ts";
import type { Environment } from "@destack/environment";

/** The milliseconds in a second, as OpenTelemetry records durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The tracker topic a loop announces releasing its locks on. */
const RELEASED_TOPIC = "control/released";

/** The retry of a failed reconciliation, doubling from a second up to 5 minutes. */
const RETRY = RetryPolicy.of({ maximumInterval: 5 * 60_000 });

/** How long a key may fail transiently before its failures are reported: a minute. */
const TRANSIENT_REPORT_MILLISECONDS = 60_000;

/** The transient failures a key reaches before a report, its retries 1, 2 and 4 s apart. */
const TRANSIENT_REPORT_ATTEMPTS = 4;

/** The most links a pass has, OpenTelemetry's default span link limit. */
const LINK_LIMIT = 128;

/** What makes a key due: a change, a listing, a retry, a released lock or its rows' next page. */
type Wake = "change" | Listing | "retry" | "release" | "page";

/** What lists a job's keys: the loop's start, its set's activation, a watch or a row come due. */
type Listing = "start" | "activation" | "watch" | "time";

/** A due key: when it is due, what made it so, and the spans of the transactions that did. */
interface DueKey {
    /** The time the key is due at, in UTC epoch milliseconds. */
    at: number;
    /** What set the time the key is due at. */
    wake: Wake;
    /** The spans of the transactions whose changes made it due. */
    readonly links: Link[];
}

/** The most pending rows one reconciliation takes: 1,000 rows of about 0.5 KB, half a megabyte. */
const KEY_ROWS = 1000;

/** The fields of a key, as its canonical JSON text has them. */
const KEY_FIELDS = schema.record(schema.string(), Scalar);

/** A projection's key, its key field's value as text. */
const ProjectionKey = schema.union([schema.string(), schema.number()]).transform(String);

/** The most compiled stored conditions a loop keeps: 10,000 at about 1 KB each, about 10 MB. */
const STORED_CONDITIONS = 10_000;

/** The key of a cursor's one checkpoint. */
const ONE_CHECKPOINT = "";

/** Microseconds per millisecond. */
const MICROSECONDS_PER_MILLISECOND = 1000;

/** The prefix of the placeholders that read the loop's time, followed by their scale and offset. */
const NOW = "now";

/** The time a pending condition compares a field to, at an offset in milliseconds. */
export class Now extends Placeholder {
    /** The offset from the loop's time, in milliseconds. */
    readonly offset: number;
    /** The compared field's units per millisecond: 1 for milliseconds, 1,000 for microseconds. */
    readonly scale: number;

    /** Read the loop's time at an offset, in a field's units. */
    constructor(offset = 0, scale = 1) {
        super(`${NOW}${String(scale)}_${String(offset)}`);
        this.offset = offset;
        this.scale = scale;
    }

    /** Read the loop's time a duration earlier. */
    minus(duration: Duration): Now {
        return new Now(this.offset - Duration.milliseconds(duration), this.scale);
    }

    /** Read the loop's time a duration later. */
    plus(duration: Duration): Now {
        return new Now(this.offset + Duration.milliseconds(duration), this.scale);
    }

    /** Read the loop's time in microseconds, for fields such as event times. */
    microseconds(): Now {
        return new Now(this.offset, MICROSECONDS_PER_MILLISECOND);
    }

    /** Bind the time at a loop's time in the field's units. */
    at(time: number): number {
        return (time + this.offset) * this.scale;
    }

    /** Read the loop's time at which a field's value comes due, inclusive or after it. */
    due(value: number, isInclusive: boolean): number {
        return isInclusive
            ? Math.ceil(value / this.scale) - this.offset
            : Math.floor(value / this.scale) - this.offset + 1;
    }
}

/** The loop's time, which pending conditions compare time fields to. */
export const now = new Now();

/** The fields that group a controller's rows into one key, in JSON form. */
export type KeyFields = Readonly<Record<string, Scalar>>;

/** Another table whose changed rows wake the keys they reference, along a path of references. */
export interface Watch<Watched extends Table = Table> {
    /** The watched table. */
    readonly table: Watched;
    /** The relations from a changed row to the rows holding the keys, none for the row itself. */
    readonly path?: readonly Relation[];
    /** The job's columns equal to the reached rows' columns, its key fields by name by default. */
    readonly on?: Readonly<Record<string, string>>;
    /** The job's rows with a stored condition the changed row must match, as stored queries. */
    readonly matches?: {
        /** The column with the stored condition. */
        readonly column: string;
        /** The job's rows that count, every row when absent. */
        readonly where?: Condition;
    };
    /** Whether a change reaching a row lists the job again instead of waking keys. */
    readonly isListing?: boolean;
    /** The changed rows that wake keys, every row when absent. */
    readonly where?: Condition;
    /** The fields whose change wakes keys, any field when absent. */
    readonly fields?: readonly string[];
    /** Whether observations of its rows wake keys too, as an owner waits for what it owns. */
    readonly isObserving?: boolean;
}

/** What a controller does with its pending rows: converge them, or hold work while they stay. */
type Work<Definition extends Table> =
    | {
          /** Converge one key's pending rows and return, writing them out of pending. */
          reconcile(
              rows: readonly Select<Definition>[],
              reconciliation: Reconciliation,
          ): Promise<void>;
      }
    | {
          /** Hold one key's work in the process while its rows stay pending, until stopped. */
          follow(rows: readonly Select<Definition>[], following: Following): Promise<void>;
      };

/** A table's reactive behavior: the rows with work, and what to do with them. */
export type Controller<Definition extends Table = Table> = {
    /** The controller's name in reports and locks. */
    readonly name: string;
    /** The rows with the work. */
    readonly table: Definition;
    /** The rows with work now, which may compare time fields to `now`. */
    readonly pending: Condition<RowImage<Definition>>;
    /** The fields grouping rows into one key, the primary key's by default. */
    readonly key?: readonly (keyof RowImage<Definition> & string)[];
    /** Other tables whose changed rows select keys. */
    readonly watches?: readonly Watch[];
    /** The most keys it works on at once, one by default. */
    readonly concurrency?: number;
} & Work<Definition>;

/** Rows derived from inputs by key, absent ones included, projected again as the inputs change. */
export interface Projection {
    /** The projection's name in reports and locks. */
    readonly name: string;
    /** The key's field name, which its watches map. */
    readonly key: string;
    /** The inputs whose changed rows touch keys. */
    readonly watches: readonly Watch[];
    /** Every key: the distinct values of one input's column. */
    readonly all: {
        /** The input defining the keys. */
        readonly table: Table;
        /** Its column holding the key. */
        readonly column: string;
        /** Its rows that define keys, every row when absent. */
        readonly where?: Condition;
    };
    /** Project one key from its inputs as they are now. */
    readonly project: (key: string, reconciliation: Reconciliation) => Promise<void>;
    /** The most keys it projects at once, one by default. */
    readonly concurrency?: number;
}

/** A position in the log: some tables' changes past one checkpoint, each taken once. */
export interface Cursor extends LogCursor {
    /** Whether hosts share it by claiming pages, so a pass reads again for untaken changes. */
    readonly isClaimed?: boolean;
    /** The read tables whose observations wake it too, none by default. */
    readonly observed?: readonly Table[];
    /** How long its first untaken change waits before it is taken, in ms. */
    readonly batch?: number;
    /** Take the changes past its checkpoint, moving the checkpoint past them. */
    readonly take: (reconciliation: Reconciliation) => Promise<void>;
    /** Write what it deferred once the loop goes idle or stops, such as its checkpoint. */
    readonly settle?: () => Promise<void>;
}

/** What a loop runs: controllers over rows, projections over inputs, and cursors over the log. */
export interface ControllerSet {
    /** The controllers converging or following rows. */
    readonly controllers: readonly Controller[];
    /** The projections deriving rows from inputs by key. */
    readonly projections: readonly Projection[];
    /** The cursors taking changes past their checkpoints. */
    readonly cursors: readonly Cursor[];
    /** Wait until the set may run. */
    readonly activation?: (signal: AbortSignal) => Promise<void>;
}

/** One job of a loop: a controller, a projection or a cursor, each locked and retried alike. */
type Job = Controller | Projection | Cursor;

/** One running reconciliation of a key. */
export interface Reconciliation {
    /** Abort once the loop stops or the key leaves pending. */
    readonly signal: AbortSignal;
    /** Wait for the key's rows to change again while it reconciles. */
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

/** A key's backoff: its failures in a row, the last failure and whether it was reported. */
interface Backoff {
    /** The failures in a row. */
    readonly count: number;
    /** The last failure. */
    readonly failure: Failure;
    /** Whether the loop reported the last failure, or an earlier one of the same reason. */
    readonly isReported: boolean;
}

/** A comparison of a field to the loop's time, which turns true as time passes. */
interface TimeTerm {
    /** The compared field. */
    readonly field: string;
    /** Whether the field may equal the time. */
    readonly isInclusive: boolean;
    /** The compared time. */
    readonly now: Now;
}

/** A controller compiled for the loop: its conditions, time terms and key fields. */
interface Compiled {
    /** The declared controller. */
    readonly controller: Controller;
    /** The pending rows, matched in memory with the loop's time bound. */
    readonly pending: Match;
    /** The comparisons to the loop's time. */
    readonly terms: readonly TimeTerm[];
    /** The comparisons of each OR branch with the top's, or the top's alone without branches. */
    readonly branches: readonly (readonly TimeTerm[])[];
    /** The placeholders of the loop's time the condition reads, by name with their offsets. */
    readonly times: ReadonlyMap<string, Now>;
    /** The fields grouping rows into one key. */
    readonly key: readonly string[];
}

/** Run controllers over one database. */
export class ControlLoop {
    /** The tables a loop keeps beside its controllers' on a shared database: their locks. */
    static readonly tables: readonly Table[] = [lock];

    /** The database with the controllers' rows and the log that selects keys. */
    readonly database: DatabaseConnection;
    /** The stored conditions of matching watches, compiled by table and JSON. */
    readonly #conditions = new Map<string, Match>();
    /** The job sets. */
    readonly sets: readonly ControllerSet[];
    /** Every set's jobs. */
    readonly #control: ControllerSet;
    /** The jobs waiting for activation. */
    readonly #inactive = new Set<Job>();
    /** Settles after the first listing. */
    readonly #started = Promise.withResolvers<void>();
    /** The controllers, projections and cursors as one list of jobs. */
    readonly #jobs: readonly Job[];
    /** Each controller compiled, by controller. */
    readonly #compiled: Map<Job, Compiled>;
    /** Report a failed reconciliation. */
    readonly #report: (controller: Job, key: string, error: unknown) => void;
    /** The loop's spans and captures, on the database's telemetry. */
    readonly #instruments: TelemetryScope;
    /** The wait of each pass for its key's lock, by job and kind. */
    readonly #waits;
    /** The time each follow took to catch up, by job. */
    readonly #catchUps;
    /** The keys a table's changes made due or due sooner, by job and table. */
    readonly #keyWakes;
    /** The host clock the waits and catch-ups read. */
    readonly #monotonic: Environment["telemetry"]["monotonic"];
    /** How a transiently failed key retries. */
    readonly #retry: RetryPolicy;
    /** The keys due, by controller and key. */
    readonly #due = new Map<Job, Map<string, DueKey>>();
    /** The due keys, earliest first. */
    readonly #queue = new DueQueue();
    /** When each controller lists its rows again as the earliest waiting one comes due. */
    readonly #wakes = new Map<Job, number>();
    /** The consecutive failures by controller and key. */
    readonly #failures = new Map<Job, Map<string, Backoff>>();
    /** When each key first failed transiently in its current run of failures. */
    readonly #transient = new Map<Job, Map<string, number>>();
    /** The keys reconciling now by controller with their abort controllers. */
    readonly #running = new Map<Job, Map<string, AbortController>>();
    /** Each controller's latest listing, which the next one waits for. */
    readonly #listing = new Map<Job, Promise<void>>();
    /** The running reconciliations and follows. */
    readonly #reconciling = new Set<Promise<void>>();
    /** The running keys not settled yet, a follow settling once synced. */
    readonly #unsettled = new Set<symbol>();
    /** The tracker whose instance locks each key, when other connections share the database. */
    readonly #tracker?: Tracker;
    /** The lock of each controller, when other connections share the database. */
    readonly #locks = new Map<Job, Lock>();
    /** Whether the loop leads each controller, settled once its lock is taken or refused. */
    readonly #leading = new Map<Job, Promise<boolean>>();
    /** The keys waiting for another holder's lock, run again once it releases or stops. */
    readonly #waiting = new Map<Job, Set<string>>();
    /** The count of other holders' releases and stops, which a refused take compares. */
    #releases = 0;
    /** The host's wake-up for the earliest due key. */
    readonly #alarm?: Alarm;
    /** The clock keys come due at, the database's. */
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
    /** The logged tables the controllers read and watch, which the follow reads, one per name. */
    readonly #watched: readonly Table[];
    /** Whether the loop stopped, after which nothing more runs. */
    #isStopped = false;
    /** The waiters for a change of each running key, by controller. */
    readonly #changes = new Map<Job, Map<string, (() => void)[]>>();
    /** Aborts once work arrives after the worker last looked for it. */
    #woken = new AbortController();

    /** Create the loop. */
    constructor(
        database: DatabaseConnection,
        sets: readonly ControllerSet[],
        options: {
            /** Report a failed reconciliation, beside the loop recording it. */
            readonly report: (
                controller: Controller | Projection | Cursor,
                key: string,
                error: unknown,
            ) => void;
            /** How a transiently failed key retries. */
            readonly retry?: Partial<RetryPolicy>;
            /** Set a wake-up for the earliest due key, so an evicted instance runs it. */
            readonly alarm?: Alarm;
            /** How long a key may fail transiently before reporting. */
            readonly transientFor?: number;
        },
    ) {
        // start idle
        this.database = database;
        this.sets = sets;
        const control: ControllerSet = {
            controllers: sets.flatMap((set) => set.controllers),
            projections: sets.flatMap((set) => set.projections),
            cursors: sets.flatMap((set) => set.cursors),
        };
        this.#control = control;
        for (const set of sets.filter((each) => each.activation !== undefined)) {
            for (const job of [...set.controllers, ...set.projections, ...set.cursors]) {
                this.#inactive.add(job);
            }
        }
        const jobs: readonly Job[] = [
            ...control.controllers,
            ...control.projections,
            ...control.cursors,
        ];
        this.#jobs = jobs;
        const instruments = TelemetryScope.of(
            import.meta.destack.package,
            database.state.environment.telemetry,
        );
        this.#instruments = instruments;
        const passed = {
            "destack.controller.name": jobs.map((job) => job.name),
            "destack.controller.kind": ["reconcile", "follow", "project", "take"],
        };
        this.#monotonic = database.state.environment.telemetry.monotonic;
        this.#waits = instruments.metric.histogram("destack.controller.wait_time", {
            unit: "s",
            attributes: passed,
        });
        this.#catchUps = instruments.metric.histogram("destack.controller.catch_up.duration", {
            unit: "s",
            attributes: passed,
        });
        const watched = jobs.flatMap((job) =>
            "watches" in job ? (job.watches ?? []).map((watch) => watch.table) : [],
        );
        const owned = control.controllers.map((controller) => controller.table);
        this.#keyWakes = instruments.metric.counter("destack.controller.wakes", {
            unit: "{wake}",
            attributes: {
                "destack.controller.name": passed["destack.controller.name"],
                "db.collection.name": [
                    ...new Set([...watched, ...owned].map((table) => table[TABLE].sqlName)),
                ],
            },
        });
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
        this.#clock = database.state.environment.clock;
        this.#transientFor = options.transientFor ?? TRANSIENT_REPORT_MILLISECONDS;
        if (options.alarm !== undefined) {
            this.#alarm = options.alarm;
        }

        // lock each controller as this loop when instances share the database
        if (database.isShared) {
            const { tracker } = database;
            const holder = Uuid.v4(database.state.environment.random);
            this.#tracker = tracker;
            for (const controller of jobs) {
                const name = ControlLoop.#lockName(controller);
                this.#locks.set(controller, new Lock(database, name, tracker.instance, holder));
            }
        }

        // refuse a watch with both a path and stored conditions
        for (const job of jobs.filter((each) => !ControlLoop.#isCursor(each))) {
            const watches = ControlLoop.#isCursor(job) ? [] : (job.watches ?? []);
            if (watches.some((watch) => watch.matches !== undefined && watch.path !== undefined)) {
                throw new TypeError(`job '${job.name}' matches stored conditions along a path`);
            }
        }

        // compile each controller, and read the logged tables they read once per name
        this.#compiled = new Map(
            control.controllers.map((controller) => [controller, ControlLoop.#compile(controller)]),
        );
        const read = jobs.flatMap((each) => ControlLoop.#tables(each));
        this.#watched = [
            ...new Map(read.map((table) => [table[TABLE].sqlName, table])).values(),
        ].filter((table) => table[TABLE].retention !== "none");
        for (const controller of jobs) {
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

        // run the waiting keys again once another holder releases its locks or stops, or the tracker syncs
        const tracker = this.#tracker;
        const unwatch = [
            tracker?.watchStopped(() => this.#requeueWaiting()),
            tracker?.listen(RELEASED_TOPIC, () => this.#requeueWaiting()),
        ];
        void tracker?.waitSynced().then(() => {
            if (!this.#isStopped) {
                this.#requeueWaiting();
            }
        });
        try {
            // wait for every loop to end, its running reconciliations included
            await Promise.all(
                [
                    this.#listen(running),
                    this.#follow(running),
                    ...this.sets.map((set) => this.#activate(set, running)),
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
            // write what the cursors deferred while this instance still holds their locks
            if (!this.database.state.isClosing) {
                await this.#settleCursors();
            }

            // leave a closing database's locks for the next instance to take over
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
        const at = this.#clock.now();
        let due = 0;
        for (const [controller, keys] of this.#due) {
            const running = found(this.#running, controller);
            for (const [key, entry] of keys) {
                due += entry.at <= at && !running.has(key) ? 1 : 0;
            }
        }

        return due + this.#unsettled.size;
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

    /** Compile a controller: its condition, the comparisons to the loop's time and its key. */
    static #compile(controller: Controller): Compiled {
        // find the comparisons to the loop's time at the top of the condition
        const top = ControlLoop.#termsOf(controller.pending);

        // find each OR branch's own comparisons, which join the top's
        const either: unknown = controller.pending["OR"];
        const branches = (Array.isArray(either) ? either : [])
            .filter(Condition.is)
            .map((branch) => [...top, ...ControlLoop.#termsOf(branch)]);
        const terms = [...top, ...branches.flat().filter((term) => !top.includes(term))];

        // refuse a reconcile of every row, which has no work to tell apart
        if ("reconcile" in controller && Object.keys(controller.pending).length === 0) {
            throw new TypeError(
                `controller '${controller.name}' reconciles every row: declare the rows pending`,
            );
        }

        // refuse the loop's time anywhere but a field's upper bound at the top of the condition
        if (ControlLoop.#countNows(controller.pending) !== terms.length) {
            throw new TypeError(
                `controller '${controller.name}' compares now outside a field's lt or lte at the top of its pending condition or of its OR's branches`,
            );
        }

        return {
            controller,
            pending: ControlLoop.#match(controller.table, controller.pending),
            terms,
            branches: branches.length === 0 ? [top] : branches,
            times: new Map(terms.map((term) => [term.now.placeholder, term.now])),
            key: controller.key ?? controller.table[TABLE].key,
        };
    }

    /** Read the comparisons to the loop's time at the top of a condition. */
    static #termsOf(condition: Readonly<Record<string, unknown>>): TimeTerm[] {
        return Object.entries(condition).flatMap(([field, entry]) =>
            Condition.is(entry)
                ? Object.entries<unknown>(entry).flatMap(([operator, value]) =>
                      (operator === "lte" || operator === "lt") && value instanceof Now
                          ? [{ field, isInclusive: operator === "lte", now: value }]
                          : [],
                  )
                : [],
        );
    }

    /** Declare the controller deleting a table's expired rows, a page at a time. */
    static expiry(table: Table, database: DatabaseConnection): Controller {
        // delete the rows whose expiry passed, refusing a table without one
        const { expiry, key } = table[TABLE];
        if (expiry === undefined) {
            throw new TypeError(`table '${table[TABLE].name}' has no expiry column`);
        }

        return {
            name: `expiry:${table[TABLE].sqlName}`,
            table,
            pending: { [expiry]: { lte: now } },
            key: [],
            reconcile: async (rows) => {
                // delete the page's rows still expired, in one statement
                const matches = rows.map((row) => {
                    const equalities = key.map((field) =>
                        eq(
                            table[TABLE].column(field),
                            present(row[field], `the key field '${field}'`),
                        ),
                    );

                    return and(nth(equalities, 0), ...equalities.slice(1));
                });
                const expired = lte(
                    table[TABLE].column(expiry),
                    database.state.environment.clock.now(),
                );
                const where = and(or(nth(matches, 0), ...matches.slice(1)), expired);
                await database.transaction(
                    async (transaction) => await transaction.delete(table).where(where),
                );
            },
        };
    }

    /** Report whether a job is a cursor over the log. */
    static #isCursor(controller: Job): controller is Cursor {
        return "take" in controller;
    }

    /** Report whether a job is a projection over inputs. */
    static #isProjection(job: Job): job is Projection {
        return "project" in job;
    }

    /** List the tables a job reads: its own and watched tables, or a cursor's. */
    static #tables(job: Job): readonly Table[] {
        if (ControlLoop.#isCursor(job)) {
            return job.tables;
        }
        const own = ControlLoop.#isProjection(job) ? job.all.table : job.table;

        return [own, ...(job.watches ?? []).map((watch) => watch.table)];
    }

    /** Count the comparisons to the loop's time anywhere in a condition. */
    static #countNows(condition: unknown): number {
        if (condition instanceof Now) {
            return 1;
        } else if (typeof condition !== "object" || condition === null) {
            return 0;
        }

        return Object.values(condition).reduce<number>(
            (count, value) => count + ControlLoop.#countNows(value),
            0,
        );
    }

    /** Compile a condition on a table for matching rows in memory. */
    static #match(table: Table, condition: Condition): Match {
        return Predicate.compile(Condition.resolve(condition, Namespace.fields(table)), table);
    }

    /** Bind the loop's time at its offsets, by placeholder name, or past every time with none. */
    #bound(compiled: Compiled, at: number | undefined): Readonly<Record<string, Scalar>> {
        if (at === undefined) {
            return Object.fromEntries(
                [...compiled.times.keys()].map((name) => [name, Number.MAX_SAFE_INTEGER]),
            );
        }

        return Object.fromEntries([...compiled.times].map(([name, time]) => [name, time.at(at)]));
    }

    /** Report whether a row matches a compiled condition with the loop's time bound. */
    #matches(compiled: Compiled, match: Match, row: Row, at: number | undefined): boolean {
        const placeholders = this.#bound(compiled, at);

        return (
            match({
                field: (name) => row[name],
                placeholder: (name) => placeholders[name] ?? Predicate.unbound(name),
                exists: () => undefined,
                search: () => undefined,
            }) === true
        );
    }

    /** Read a row's key: its key fields in JSON form, canonically. */
    static #keyOf(compiled: Compiled, row: Row): string {
        const definition = compiled.controller.table[TABLE];
        const fields = Object.fromEntries(
            compiled.key.map((field) => [
                field,
                definition.column(field).definition.toJson(row[field] ?? null),
            ]),
        );

        return canonicalize(fields);
    }

    /** Wait until the log listens to other writers' commits, failing as listening fails. */
    async #listen(signal: AbortSignal): Promise<void> {
        await Promise.race([this.database.log.listening(), waitAborted(signal)]);
        this.#isListening = !signal.aborted;
        this.#wake();
    }

    /** List every job's keys, then follow the logged tables from the listing on. */
    async #follow(signal: AbortSignal): Promise<void> {
        // list every job at each start, as its process may have changed
        const active = () => this.#jobs.filter((job) => !this.#inactive.has(job));
        let listed = active();
        while (!signal.aborted) {
            // read the position before listing
            const after = (await this.database.log.position()).sequence;
            for (const controller of listed) {
                await this.#list(controller, "start");
            }
            this.#followed = after;
            this.#isListed = true;
            this.#started.resolve();
            this.#wake();
            const tables = this.#watched;
            if (tables.length === 0) {
                return;
            }

            // follow the changes after the listing until stopped or asked to list again
            this.#relisting = new AbortController();
            const following = AbortSignal.any([signal, this.#relisting.signal]);
            try {
                for await (const page of this.database.log.follow({ tables, after }, following)) {
                    for (const change of page.changes) {
                        await this.#changed(change);
                    }

                    // record the consumed sequence for idle waiters, and let other tasks run
                    this.#followed = Math.max(this.#followed, page.sequence);
                    this.#wake();
                    await this.database.state.environment.scheduler.yield();
                }
            } catch (error) {
                // list again after compaction
                if (!DatabaseError.is(error, "CHANGES_NOT_FOUND")) {
                    throw error;
                }
            }
            listed = active();
        }
    }

    /** Run a set's jobs once activated. */
    async #activate(set: ControllerSet, signal: AbortSignal): Promise<void> {
        // wait for activation
        const { activation } = set;
        if (activation === undefined) {
            return;
        }
        await activation(signal);
        await Promise.race([this.#started.promise, waitAborted(signal)]);
        if (signal.aborted) {
            return;
        }

        // mark the loop busy
        const unsettled = Symbol("activation");
        this.#unsettled.add(unsettled);
        try {
            // activate the jobs and list them
            const jobs = [...set.controllers, ...set.projections, ...set.cursors];
            for (const job of jobs) {
                this.#inactive.delete(job);
            }
            for (const job of jobs) {
                await this.#list(job, "activation");
            }
        } finally {
            this.#unsettled.delete(unsettled);
            this.#wake();
        }
    }

    /** Select the keys a committed change makes pending, wakes or changes. */
    async #changed(change: Change): Promise<void> {
        // read the change's table, time and active projections
        const name = change.table[TABLE].sqlName;
        const at = this.#clock.now();
        const projections = this.#control.projections.filter((each) => !this.#inactive.has(each));

        // leave an observation to the jobs reading observations, as they alone wait for it
        const isObservation = Change.isObservation(change);

        // take a read table's change, due once the cursor's batch passed
        for (const cursor of this.#control.cursors.filter((each) => !this.#inactive.has(each))) {
            const isRead = (tables: readonly Table[]) =>
                tables.some((table) => table[TABLE].sqlName === name);
            if (isRead(cursor.tables) && (!isObservation || isRead(cursor.observed ?? []))) {
                this.#enqueue(cursor, ONE_CHECKPOINT, at + (cursor.batch ?? 0), change);
            }
        }
        for (const compiled of this.#compiled.values()) {
            // take a change of the controller's own rows
            const { controller } = compiled;
            if (this.#inactive.has(controller)) {
                continue;
            }
            if (controller.table[TABLE].sqlName === name) {
                // tell the running key of the row before its change, then select the row after it
                //  unless an observation leaves it in a pending condition that reads no fields
                const before = Change.before(change);
                const after = Change.after(change);
                if (before !== null) {
                    this.#notify(controller, ControlLoop.#keyOf(compiled, before));
                }
                if (
                    after !== null &&
                    (!isObservation || Object.keys(controller.pending).length > 0)
                ) {
                    this.#select(compiled, after, at, change);
                }
            }

            // take the keys a watched table's change selects
            await this.#wakeWatched(controller, change, isObservation, (fields) =>
                canonicalize(fields),
            );
        }

        // take the keys a projection's watched inputs' change touches
        for (const projection of projections) {
            await this.#wakeWatched(projection, change, isObservation, (fields) =>
                ProjectionKey.parse(fields[projection.key]),
            );
        }
    }

    /** Enqueue the keys a change of a job's watched tables reaches along their references. */
    async #wakeWatched(
        job: Controller | Projection,
        change: Change,
        isObservation: boolean,
        keyOf: (fields: KeyFields) => string,
    ): Promise<void> {
        // take the watches of the changed table
        const name = change.table[TABLE].sqlName;
        const watches = (job.watches ?? []).filter(
            (watch) =>
                watch.table[TABLE].sqlName === name &&
                (!isObservation || watch.isObserving === true),
        );
        const fields = ControlLoop.#isProjection(job)
            ? [job.key]
            : (job.key ?? job.table[TABLE].key);
        const table = ControlLoop.#isProjection(job) ? job.all.table : job.table;
        const columns = ControlLoop.#isProjection(job) ? [job.all.column] : fields;
        const space = ControlLoop.#isProjection(job) ? job.all.where : undefined;
        for (const watch of watches) {
            // list the job again once a listing watch's change reaches a row
            if (watch.isListing === true) {
                if ((await this.#walk(watch, ControlLoop.#images(watch, change))).length > 0) {
                    await this.#list(job, "watch");
                }
            }
            // take the keys reached
            else {
                const reached = await this.#reached(watch, change, {
                    fields,
                    columns,
                    table,
                    ...(space === undefined ? {} : { where: space }),
                });
                for (const keyFields of reached) {
                    const key = keyOf(keyFields);
                    this.#notify(job, key);
                    this.#enqueue(job, key, this.#clock.now(), change);
                }
            }
        }
    }

    /** Read the key fields a watched change reaches: its rows before and after, along the path. */
    async #reached(
        watch: Watch,
        change: Change,
        job: {
            /** The job's key fields. */
            readonly fields: readonly string[];
            /** The columns of the job's table holding them, in order. */
            readonly columns: readonly string[];
            /** The table with the job's keys. */
            readonly table: Table;
            /** The rows of the table that hold keys, every row when absent. */
            readonly where?: Condition;
        },
    ): Promise<readonly KeyFields[]> {
        // take the images the watch reads
        const { fields: keyFields, table } = job;
        const images = ControlLoop.#images(watch, change);
        let rows = images;

        // walk the path to its rows
        rows = await this.#walk(watch, rows);

        // take the job's rows whose stored condition a changed image matches
        const { matches } = watch;
        if (matches !== undefined) {
            const stored = await this.#where(table, matches.where);
            rows = stored.filter((row) => {
                const match = this.#stored(watch.table, row[matches.column]);

                return images.some((image) => ControlLoop.#isMatch(match, image));
            });
        }

        // take the values the job's columns equal
        const { columns } = job;
        const on = watch.on ?? Object.fromEntries(columns.map((column) => [column, column]));
        const matched: Row[] = [];
        for (const row of rows) {
            const values: Record<string, ColumnValue> = {};
            for (const [field, column] of Object.entries(on)) {
                const value = row[column];
                if (value !== null && value !== undefined) {
                    values[field] = value;
                }
            }
            if (Object.keys(values).length === Object.keys(on).length) {
                matched.push(values);
            }
        }

        // read the key columns from the values, else from the job's rows within its key space
        const isKeyRow = job.where === undefined || watch.table === table;
        const isKeyed = isKeyRow && columns.every((column) => Object.hasOwn(on, column));
        const keyed =
            isKeyed || matched.length === 0
                ? matched
                : await this.#equal(table, matched, job.where);
        const keys = keyed.map((row) =>
            KEY_FIELDS.parse(
                Object.fromEntries(
                    keyFields.map((field, index) => [field, row[nth(columns, index)]]),
                ),
            ),
        );

        return [...new Map(keys.map((each) => [canonicalize(each), each])).values()];
    }

    /** Compile a stored condition on a watched table once, by its JSON. */
    #stored(table: Table, value: unknown): Match {
        // reuse the condition compiled before
        const name = `${table[TABLE].sqlName} ${canonicalize(value)}`;
        const cached = this.#conditions.get(name);
        if (cached !== undefined) {
            return cached;
        }

        // compile it, starting over once the cache is full
        const match = ControlLoop.#match(table, Condition.schema.parse(value));
        if (this.#conditions.size >= STORED_CONDITIONS) {
            this.#conditions.clear();
        }
        this.#conditions.set(name, match);

        return match;
    }

    /** Walk a watch's path from some rows to the rows its relations reach. */
    async #walk(watch: Watch, images: readonly Row[]): Promise<readonly Row[]> {
        // start at the images
        let rows = images;
        let reaching: Table = watch.table;
        for (const relation of watch.path ?? []) {
            // stop once no row is left
            if (rows.length === 0) {
                return [];
            }

            // read the related rows
            const reader = new ModelQueryBuilder(this.database, reaching, this.database.relations);
            rows = await reader.related(relation, rows);
            reaching = relation.table;
        }

        return rows;
    }

    /** Read the images of a change a watch reads, none once no watched field changed. */
    static #images(watch: Watch, change: Change): readonly Row[] {
        // leave a change of no watched field
        const before = Change.before(change);
        const after = Change.after(change);
        const fields = watch.fields;
        if (
            fields !== undefined &&
            before !== null &&
            after !== null &&
            fields.every(
                (field) =>
                    canonicalize(before[field] ?? null) === canonicalize(after[field] ?? null),
            )
        ) {
            return [];
        }

        // take the images the watch's condition matches
        const where =
            watch.where === undefined ? undefined : ControlLoop.#match(watch.table, watch.where);

        return [before, after].filter(
            (image): image is Row =>
                image !== null && (where === undefined || ControlLoop.#isMatch(where, image)),
        );
    }

    /** Read a job's rows matching a condition. */
    async #where(table: Table, where: Condition | undefined): Promise<readonly Row[]> {
        // read the rows matching the condition
        const condition =
            where === undefined
                ? undefined
                : Predicate.render(
                      Condition.resolve(where, Namespace.fields(table)),
                      Predicate.bind(table, {}),
                  );

        return await this.database.select().from(table).where(condition);
    }

    /** Read a job's rows equal to some values by column, within the rows that hold keys. */
    async #equal(
        table: Table,
        values: readonly Row[],
        where: Condition | undefined,
    ): Promise<readonly Row[]> {
        // read the rows equal to any of the values
        const matches = values.map((row) => {
            const equalities = Object.entries(row).map(([field, value]) =>
                eq(table[TABLE].column(field), value),
            );

            return and(nth(equalities, 0), ...equalities.slice(1));
        });

        // keep the rows holding keys
        const holding =
            where === undefined
                ? undefined
                : Predicate.render(
                      Condition.resolve(where, Namespace.fields(table)),
                      Predicate.bind(table, {}),
                  );

        return await this.database
            .select()
            .from(table)
            .where(and(or(nth(matches, 0), ...matches.slice(1)), holding));
    }

    /** Report whether a row matches a compiled condition without the loop's time. */
    static #isMatch(match: Match, row: Row): boolean {
        return (
            match({
                field: (name) => row[name],
                placeholder: (name) => Predicate.unbound(name),
                exists: () => undefined,
                search: () => undefined,
            }) === true
        );
    }

    /** Make a row's key pending, or wake the controller once its waiting row comes due. */
    #select(compiled: Compiled, row: Row, at: number, change: Change): void {
        // tell the running key its rows changed
        const { controller } = compiled;
        const key = ControlLoop.#keyOf(compiled, row);
        this.#notify(controller, key);

        // take a pending row's key, or wake at a waiting row's due time
        if (this.#matches(compiled, compiled.pending, row, at)) {
            this.#enqueue(controller, key, at, change);
        } else if (
            compiled.terms.length > 0 &&
            this.#matches(compiled, compiled.pending, row, undefined)
        ) {
            const due = ControlLoop.#dueOf(compiled, row);
            if (due !== undefined) {
                this.#wakeAt(controller, due);
            }
        }
    }

    /** Read when a waiting row turns pending: the earliest branch whose comparisons all hold. */
    static #dueOf(compiled: Compiled, row: Row): number | undefined {
        // read when each branch's comparisons all hold
        const dues = compiled.branches.flatMap((terms) => {
            const times = terms.map((term) => {
                const value = row[term.field];

                return typeof value === "number"
                    ? term.now.due(value, term.isInclusive)
                    : undefined;
            });

            return terms.length === 0 || times.some((time) => time === undefined)
                ? []
                : [Math.max(...times.filter((time) => time !== undefined))];
        });

        return dues.length === 0 ? undefined : Math.min(...dues);
    }

    /** Wake a controller to list its rows again at a time, keeping an earlier wake. */
    #wakeAt(controller: Job, at: number): void {
        const current = this.#wakes.get(controller);
        if (current === undefined || at < current) {
            this.#wakes.set(controller, at);
            this.#wake();
        }
    }

    /** List a controller's pending keys and its next wake, one listing at a time. */
    #list(controller: Job, by: Listing): Promise<void> {
        const listing = (this.#listing.get(controller) ?? Promise.resolve()).then(() =>
            this.#read(controller, by),
        );
        this.#listing.set(
            controller,
            listing.catch(() => {}),
        );

        return listing;
    }

    /** List a keyless controller's one key while any of its rows is pending. */
    async #firstPending(table: Table, where: SQL | undefined): Promise<readonly Row[]> {
        const [pending] = await this.database.select().from(table).where(where).limit(1);

        return pending === undefined ? [] : [{}];
    }

    /** Read a controller's pending keys, enqueueing those not failing, and its next due row. */
    async #read(controller: Job, by: Listing): Promise<void> {
        // take a listed cursor once its batch passes
        if (ControlLoop.#isCursor(controller)) {
            this.#wakes.delete(controller);
            this.#enqueue(
                controller,
                ONE_CHECKPOINT,
                this.#clock.now() + (controller.batch ?? 0),
                by,
            );

            return;
        }

        // take every key of a projection: the distinct values of its key-defining input's column
        if (ControlLoop.#isProjection(controller)) {
            const { table, column, where } = controller.all;
            const condition =
                where === undefined
                    ? undefined
                    : Predicate.render(
                          Condition.resolve(where, Namespace.fields(table)),
                          Predicate.bind(table, {}),
                      );
            const rows = await this.database
                .selectDistinct({ key: table[TABLE].column(column) })
                .from(table)
                .where(condition);
            const failures = found(this.#failures, controller);
            for (const row of rows) {
                const key = ProjectionKey.parse(row.key);
                if (!failures.has(key)) {
                    this.#enqueue(controller, key, this.#clock.now(), by);
                }
            }

            return;
        }

        // read the pending keys at the loop's time
        const compiled = found(this.#compiled, controller);
        const { table } = controller;
        const at = this.#clock.now();
        const placeholders = this.#bound(compiled, at);
        const columns = Object.fromEntries(
            compiled.key.map((field) => [field, table[TABLE].column(field)]),
        );
        const where = Predicate.render(
            Condition.resolve(controller.pending, Namespace.fields(table)),
            Predicate.bind(table, placeholders),
        );
        const rows =
            compiled.key.length === 0
                ? await this.#firstPending(table, where)
                : await this.database.selectDistinct(columns).from(table).where(where);
        const failures = found(this.#failures, controller);
        for (const row of rows) {
            const key = ControlLoop.#keyOf(compiled, row);
            if (!failures.has(key)) {
                this.#enqueue(controller, key, at, by);
            }
        }

        // wake at the earliest row not yet due, one aggregate per time comparison
        this.#wakes.delete(controller);
        for (const term of compiled.terms) {
            const column = table[TABLE].column(term.field);
            const bound = term.now.at(at);
            const rest = Predicate.render(
                Condition.resolve(controller.pending, Namespace.fields(table)),
                Predicate.bind(table, this.#bound(compiled, undefined)),
            );
            const [oldest] = await this.database
                .select({ value: min(column) })
                .from(table)
                .where(and(rest, term.isInclusive ? gt(column, bound) : gte(column, bound)));
            const value: unknown = oldest?.value;
            if (typeof value === "number") {
                this.#wakeAt(controller, term.now.due(value, term.isInclusive));
            }
        }
    }

    /** Enqueue a cursor with changes past its checkpoint after its batch, unplaced at once. */
    async #readUntaken(cursor: Cursor): Promise<void> {
        // leave a cursor reading no table, which only a start takes
        if (cursor.tables.length === 0) {
            return;
        }

        // read the first change past the checkpoint and whether it is placed, in one statement
        const log = sql.relation(LOG);
        const checkpoint = sql.relation(LOG_CHECKPOINT);
        const tables = sql.join(
            cursor.tables.map((table) => sql`${table[TABLE].sqlName}`),
            sql.raw(", "),
        );
        const [row] = await this.database.all(sql`
            SELECT (
                SELECT min(entry.changed_at) FROM ${log} AS entry
                WHERE entry."table" IN (${tables})
                    AND entry.sequence > coalesce(
                        (SELECT place.sequence FROM ${checkpoint} AS place WHERE place.name = ${cursor.name}),
                        -1
                    )
            ) AS first,
            (SELECT count(*) FROM ${checkpoint} AS place WHERE place.name = ${cursor.name}) AS placed
        `);

        // take an unplaced cursor at once, and one with changes once its first change's batch passed
        const isPlaced = LogColumn.integer(row?.["placed"]) > 0;
        const first = LogColumn.nullableInteger(row?.["first"]);
        if (!isPlaced) {
            this.#enqueue(cursor, ONE_CHECKPOINT, this.#clock.now(), "page");
        } else if (first !== null) {
            this.#enqueue(cursor, ONE_CHECKPOINT, first + (cursor.batch ?? 0), "page");
        }
    }

    /** Wake the controllers of unlogged tables once per batch of commits, as a work queue does. */
    async #listUnlogged(signal: AbortSignal): Promise<void> {
        // find the controllers reading or watching unlogged tables
        const unlogged = new Map(
            this.#jobs.map((controller) => [
                controller,
                ControlLoop.#tables(controller)
                    .filter((table) => table[TABLE].retention === "none")
                    .map((table) => table[TABLE].sqlName),
            ]),
        );
        if (![...unlogged.values()].some((tables) => tables.length > 0)) {
            return;
        }
        await this.database.log.waitUntil((committed) => {
            // wake each controller
            const at = this.#clock.now();
            for (const controller of this.#jobs.filter((job) => !this.#inactive.has(job))) {
                if (found(unlogged, controller).some((table) => committed.has(table))) {
                    this.#wakeAt(controller, at);
                }
            }

            return Promise.resolve(false);
        }, signal);
    }

    /** Mark a key due by a change or another wake. */
    #enqueue(controller: Job, key: string, at: number, by: Change | Exclude<Wake, "change">): void {
        // find the due key, new without a time yet
        const due = found(this.#due, controller);
        const known = due.get(key);
        const entry: DueKey = known ?? { at: Infinity, wake: "change", links: [] };
        due.set(key, entry);

        // take the earliest time and what set it, counting a key a change made due or due sooner
        if (at < entry.at) {
            entry.at = at;
            entry.wake = typeof by === "string" ? by : "change";
            this.#queue.push({ controller, key, at });
            if (typeof by !== "string") {
                this.#keyWakes.add(1, {
                    "destack.controller.name": controller.name,
                    "db.collection.name": by.table[TABLE].sqlName,
                });
            }
        }

        // link the key's pass to the span of a change's transaction, once
        const span =
            typeof by === "string" || by.transaction === null
                ? undefined
                : this.database.state.transactionSpans.get(by.transaction);
        const isLinked = entry.links.some((link) => link.context.spanId === span?.spanId);
        if (typeof by !== "string" && span !== undefined && !isLinked) {
            if (entry.links.length < LINK_LIMIT) {
                const attributes = { "db.collection.name": by.table[TABLE].sqlName };
                entry.links.push({ context: span, attributes });
            }
        }
        this.#wake();
    }

    /** Tell a running key that its rows changed. */
    #notify(controller: Job, key: string): void {
        for (const resolve of this.#changes.get(controller)?.get(key)?.splice(0) ?? []) {
            resolve();
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
            const page = await this.database.log.next({
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
            if (DatabaseError.is(error, "CHANGES_NOT_FOUND")) {
                relisting.abort();

                return false;
            }
            throw error;
        }
    }

    /** Start due keys and wakes, and sleep until more are due. */
    async #work(signal: AbortSignal): Promise<void> {
        while (!signal.aborted) {
            // list the controllers whose earliest waiting row came due
            this.#woken = new AbortController();
            const at = this.#clock.now();
            const woken = [...this.#wakes].filter(([, wake]) => wake <= at);
            if (woken.length > 0) {
                await Promise.all(woken.map(([controller]) => this.#list(controller, "time")));
                continue;
            }

            // take the earliest runnable key
            const { runnable, due, skipped } = this.#take(at);
            const wake = this.#nextWake();
            const next = earliest([due, wake]);

            // start it, holding no lock meanwhile
            if (runnable !== undefined) {
                this.#start(runnable);

                // set the wake-up due while the key runs
                await this.#schedule(this.#busy);

                // let other tasks run before the next key
                await this.database.state.environment.scheduler.yield();
            }
            // set the wake-up due while keys run, or at the next key or wake
            else {
                await this.#schedule(this.#busy ?? earliest([next, skipped]));

                // settle idle waiters once every key and change is settled
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

                // write what the cursors deferred once nothing runs, then sleep until the next key or wake is due
                if (this.#unsettled.size === 0 && skipped === undefined) {
                    await this.#settleCursors();
                }
                await this.#sleep(next === undefined ? undefined : Math.max(0, next - at), signal);
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

    /** Write what the cursors deferred, each in turn. */
    async #settleCursors(): Promise<void> {
        for (const cursor of this.#control.cursors) {
            await cursor.settle?.();
        }
    }

    /** Read the earliest wake of any controller, absent for none. */
    #nextWake(): number | undefined {
        return earliest([...this.#wakes.values()]);
    }

    /** Take the earliest runnable key, or the next due time. */
    #take(at: number): {
        readonly runnable?: Pass;
        readonly due?: number;
        readonly skipped?: number;
    } {
        // skip running keys and keys of a full controller
        const skipped: Due[] = [];
        let taken: { readonly runnable?: Pass; readonly due?: number; readonly skipped?: number } =
            {};
        for (let next = this.#next(); next !== undefined; next = this.#next()) {
            const running = found(this.#running, next.controller);
            const limit = ControlLoop.#isCursor(next.controller)
                ? 1
                : (next.controller.concurrency ?? 1);
            const isFull = running.size >= limit;
            if (next.at > at) {
                taken = { due: next.at };
                break;
            } else if (isFull || running.has(next.key)) {
                this.#queue.pop();
                skipped.push(next);
            } else {
                this.#queue.pop();
                const due = found(this.#due, next.controller);
                const { wake, links } = found(due, next.key);
                due.delete(next.key);
                taken = { runnable: { ...next, wake, links } };
                break;
            }
        }

        // queue the skipped keys again
        for (const entry of skipped) {
            this.#queue.push(entry);
        }

        // find the earliest skipped key
        const [first] = skipped;

        return first === undefined || taken.runnable !== undefined
            ? taken
            : { ...taken, skipped: first.at };
    }

    /** Start a reconciliation or follow of a key. */
    #start(work: Pass): void {
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
        const reconciling = this.#reconcile(work, stop, settle).finally(() => {
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

    /** Read a key's pending rows at the loop's time. */
    async #rows(compiled: Compiled, key: string): Promise<Select<Table>[]> {
        // match the pending rows of the key's fields at the loop's time
        const { table } = compiled.controller;
        const fields = KEY_FIELDS.parse(JSON.parse(key));
        const placeholders = this.#bound(compiled, this.#clock.now());
        const condition: Condition = {
            AND: [compiled.controller.pending, Condition.equal(fields)],
        };
        const where = Predicate.render(
            Condition.resolve(condition, Namespace.fields(table)),
            Predicate.bind(table, placeholders),
        );

        return this.database.select().from(table).where(where).limit(KEY_ROWS);
    }

    /** Run one pass of a job over a key in its span, recording its lock wait and duration. */
    #pass(work: Pass, queued: number, run: () => Promise<void>): Promise<void> {
        // record how long the pass waited for the key's lock
        const attributes = ControlLoop.#attributes(work);
        this.#waits.record((this.#monotonic.now() - queued) / MILLISECONDS_PER_SECOND, attributes);

        // run it in its span, timing a reconcile or a take, as a follow lasts until stopped
        const isFollow = attributes["destack.controller.kind"] === "follow";
        const woken = { ...attributes, "destack.wake.kind": work.wake };

        return this.#instruments.span(
            "destack.controller.pass",
            { ...woken, "destack.key": work.key },
            run,
            { duration: isFollow ? false : { attributes: woken }, links: work.links },
        );
    }

    /** Wrap a follow's report of catching up, recording how long it took to catch up once. */
    #caughtUp(work: Due, synced: () => void): () => void {
        // time the follow from its start
        const started = this.#monotonic.now();
        let isCaughtUp = false;

        return () => {
            // record the first catch-up, then report it
            if (!isCaughtUp) {
                isCaughtUp = true;
                const seconds = (this.#monotonic.now() - started) / MILLISECONDS_PER_SECOND;
                this.#catchUps.record(seconds, ControlLoop.#attributes(work));
            }
            synced();
        };
    }

    /** Name a job and its kind as its signals' attributes. */
    static #attributes(work: Due): {
        readonly "destack.controller.name": string;
        readonly "destack.controller.kind": "reconcile" | "follow" | "project" | "take";
    } {
        const { controller } = work;
        const kind = ControlLoop.#isCursor(controller)
            ? "take"
            : ControlLoop.#isProjection(controller)
              ? "project"
              : "follow" in controller
                ? "follow"
                : "reconcile";

        return { "destack.controller.name": controller.name, "destack.controller.kind": kind };
    }

    /** Reconcile one key's pending rows, or follow them until they leave, retrying failures. */
    async #reconcile(work: Pass, stop: AbortController, synced: () => void): Promise<void> {
        // reconcile under the lock
        const { controller, key } = work;
        const failures = found(this.#failures, controller);
        const stopped = stop.signal;
        try {
            const queued = this.#monotonic.now();
            await this.#locked(work, stopped, (reconciliation) =>
                this.#pass(work, queued, async () => {
                    // take a cursor's changes, then again while changes remain past its checkpoint
                    if (ControlLoop.#isCursor(controller)) {
                        await controller.take(reconciliation);
                        if (controller.isClaimed === true) {
                            await this.#readUntaken(controller);
                        }

                        return;
                    }

                    // project a projection's key from its inputs as they are now
                    if (ControlLoop.#isProjection(controller)) {
                        await controller.project(key, reconciliation);

                        return;
                    }

                    // leave a key without pending rows
                    const compiled = found(this.#compiled, controller);
                    const rows = await this.#rows(compiled, key);
                    if (rows.length === 0) {
                        return;
                    }

                    // follow until stopped or the key leaves pending, reporting once caught up
                    if ("follow" in controller) {
                        const ended = new AbortController();
                        const watched = {
                            ...reconciliation,
                            signal: AbortSignal.any([reconciliation.signal, ended.signal]),
                        };
                        const watching = this.#stopWhenSettled(compiled, key, watched, stop);
                        try {
                            const caughtUp = this.#caughtUp(work, synced);
                            await controller.follow(rows, { ...reconciliation, synced: caughtUp });
                        } finally {
                            ended.abort();
                            await watching.catch(() => undefined);
                        }
                        if (!reconciliation.signal.aborted) {
                            throw DatabaseError.of(
                                "FOLLOW_BROKEN",
                                `'${controller.name}' stopped following '${key}' without an abort`,
                            );
                        }

                        return;
                    }

                    // reconcile the rows
                    await controller.reconcile(rows, reconciliation);

                    // take the key's next page of pending rows after a full one
                    if (rows.length === KEY_ROWS) {
                        this.#enqueue(controller, key, this.#clock.now(), "page");
                    }
                }),
            );
            failures.delete(key);
            this.#transient.get(controller)?.delete(key);
        }
        // end a reconciliation the loop stopped without a failure
        catch (error) {
            if (stopped.aborted) {
                failures.delete(key);

                return;
            }

            // stop the loop on a failure the controller declares no outcome for
            if (!Failure.isDeclared(error)) {
                this.#failing.abort(
                    DatabaseError.of(
                        "CONTROLLER_BROKEN",
                        `'${controller.name}' did not reconcile '${key}'`,
                        { cause: error },
                    ),
                );

                return;
            }

            // report a failure once, a transient one once it persists
            const previous = failures.get(key);
            const failure = Failure.of(error);
            const failed = (previous?.count ?? 0) + 1;
            const isRepeated =
                previous?.isReported === true &&
                previous.failure.domain === failure.domain &&
                previous.failure.reason === failure.reason;
            const isReported = !isRepeated && this.#recordFailure(work, failure, failed);
            if (isReported) {
                this.#report(controller, key, error);
            }
            failures.set(key, { count: failed, failure, isReported: isRepeated || isReported });

            // retry a transient failure on backoff, leaving any other to the key's next change
            if (Failure.isTransient(failure)) {
                const drawn = RetryPolicy.draw(`${controller.name} ${key} ${failed}`);
                this.#enqueue(
                    controller,
                    key,
                    this.#clock.now() + RetryPolicy.interval(this.#retry, failed, () => drawn),
                    "retry",
                );
            }
        }
    }

    /** Stop a follow once a change leaves its key without pending rows. */
    async #stopWhenSettled(
        compiled: Compiled,
        key: string,
        reconciliation: Reconciliation,
        stop: AbortController,
    ): Promise<void> {
        while (!reconciliation.signal.aborted) {
            await Promise.race([reconciliation.changed(), waitAborted(reconciliation.signal)]);
            if (reconciliation.signal.aborted) {
                return;
            }
            const rows = await this.#rows(compiled, key);
            if (rows.length === 0) {
                stop.abort(
                    new Error(`'${compiled.controller.name}' no longer has '${key}' pending`),
                );

                return;
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
    #recordFailure(work: Due, failure: Failure, attempts: number): boolean {
        // report a lasting failure at once
        const at = this.#clock.now();
        const since = this.#transient.get(work.controller) ?? new Map<string, number>();
        this.#transient.set(work.controller, since);
        if (!Failure.isTransient(failure)) {
            since.delete(work.key);

            return true;
        }

        // report a transient failure once it lasted past the window over several attempts
        const first = since.get(work.key) ?? at;
        since.set(work.key, first);

        return at - first >= this.#transientFor && attempts >= TRANSIENT_REPORT_ATTEMPTS;
    }

    /** Reconcile a key while the loop leads its controller. */
    async #locked(
        work: Due,
        stopped: AbortSignal,
        reconcile: (reconciliation: Reconciliation) => Promise<void>,
    ): Promise<void> {
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

        // lead the controller, its keys sharing one take of its lock, counting the releases since
        const { controller, key } = work;
        const releases = this.#releases;
        let leading = this.#leading.get(controller);
        if (leading === undefined) {
            leading = found(this.#locks, controller).take((instance) => tracker.isLive(instance));
            this.#leading.set(controller, leading);
        }

        // run the key once the loop leads, else after another holder releases
        if (!(await leading)) {
            if (this.#leading.get(controller) === leading) {
                this.#leading.delete(controller);
            }
            // wait for the next release
            if (this.#releases === releases) {
                const waiting = this.#waiting.get(controller) ?? new Set();
                this.#waiting.set(controller, waiting.add(key));
            }
            // take the lock again at once after a release during the take
            else {
                this.#enqueue(controller, key, this.#clock.now(), "release");
            }

            return undefined;
        }

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
            if (found(this.#due, next.controller).get(next.key)?.at === next.at) {
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
            await (delay === undefined ? waitAborted(waking) : this.#clock.sleep(delay, waking));
        } catch (error) {
            // end the sleep on a wake or a stop
            if (!waking.aborted) {
                throw error;
            }
        }
    }

    /** Run every waiting key again. */
    #requeueWaiting(): void {
        this.#releases += 1;
        for (const [controller, keys] of this.#waiting) {
            for (const key of keys) {
                this.#enqueue(controller, key, this.#clock.now(), "release");
            }
            keys.clear();
        }
    }

    /** Read the lock name of a controller. */
    static #lockName(controller: Job): string {
        return `controller:${controller.name}`;
    }
}

/** Take the earliest of some times, absent for none. */
function earliest(times: readonly (number | undefined)[]): number | undefined {
    const known = times.filter((time) => time !== undefined);

    return known.length === 0 ? undefined : Math.min(...known);
}

/** One key due at a time. */
interface Due {
    /** The controller or cursor working on the key. */
    readonly controller: Job;
    /** The key. */
    readonly key: string;
    /** The time the key is due at, in UTC epoch milliseconds. */
    readonly at: number;
}

/** One pass of a job over a key taken from the due keys, with what made the key due. */
interface Pass extends Due {
    /** What set the time the key was due at. */
    readonly wake: Wake;
    /** The spans of the transactions whose changes made the key due. */
    readonly links: readonly Link[];
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
        const first = heap[0];
        const last = heap.pop();
        if (heap.length === 0 || last === undefined) {
            return first;
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
                return first;
            }
            [heap[smallest], heap[index]] = [nth(heap, index), nth(heap, smallest)];
            index = smallest;
        }
    }
}
