import { expect, onTestFinished, Recording, refusal, test } from "@destack/test";
import {
    Condition,
    defineRelations,
    defineTable,
    eq,
    json,
    inArray,
    integer,
    lock,
    type Projection,
    type Table,
    text,
} from "../index.ts";
import { TestDatabase } from "../test/index.ts";
import { testSimulation } from "@destack/test/world";
import { testHost } from "@destack/test/host";
import { canonicalize, nth, found } from "@destack/schema";
import { ControlLoop, type Controller, now, type Cursor } from "./control.ts";
import { waitAborted, yieldToEventLoop } from "@destack/environment";
import { SimulatedTelemetry, Simulation } from "@destack/environment/simulation";
import { Failure } from "@destack/error";
import { TestTelemetry } from "@destack/telemetry/test";

/** The jobs a controller reconciles: open ones, each due at a time. */
const job = defineTable(
    "controller_job",
    {
        /** The job's name. */
        id: text("id").primaryKey(),
        /** The space with the job. */
        scope: text("scope").notNull(),
        /** Whether the job is open or done. */
        status: text("status").notNull(),
        /** When the job is due, in UTC epoch milliseconds. */
        dueAt: integer("due_at").notNull(),
    },
    { log: {} },
);

/** The jobs one burst of a synchronous database reconciles, unlogged. */
const queuedJob = defineTable("controller_queued_job", {
    /** The job's name. */
    id: text("id").primaryKey(),
    /** Whether the job is open. */
    status: text("status").notNull(),
});

/** The keys one burst reconciles. */
const BURST_KEYS = 20;

/** The reads each reconciliation makes, which a synchronous database answers without yielding. */
const BURST_READS = 20;

/** The synchronous work each reconciliation does beside its reads, such as decoding, in ms. */
const BURST_WORK_MILLISECONDS = 2;

/** The interval of the timer that probes the event loop, in milliseconds. */
const PROBE_MILLISECONDS = 1;

/** The jobs table defined anew, as a module loaded again defines it. */
const reloadedJob = defineTable(
    "controller_job",
    {
        /** The job's name. */
        id: text("id").primaryKey(),
        /** The space with the job. */
        scope: text("scope").notNull(),
        /** Whether the job is open or done. */
        status: text("status").notNull(),
        /** When the job is due, in UTC epoch milliseconds. */
        dueAt: integer("due_at").notNull(),
    },
    { log: {} },
);

/** Logged notes whose changes select the jobs of their space. */
const note = defineTable(
    "controller_note",
    {
        /** The note's name. */
        id: text("id").primaryKey(),
        /** The space with the note. */
        scope: text("scope").notNull(),
    },
    { log: {} },
);

/** Logged orders, an update leaving their generation being status. */
const order = defineTable(
    "controller_order",
    {
        /** The order's name. */
        id: text("id").primaryKey(),
        /** The space with the order. */
        scope: text("scope").notNull(),
        /** The count of the order's desired state's changes. */
        generation: integer("generation").notNull(),
        /** The order's observed state. */
        state: text("state").notNull(),
    },
    { log: { generation: "generation" } },
);

/** Unlogged tickets a controller reads. */
const ticket = defineTable("controller_ticket", {
    /** The ticket's name. */
    id: text("id").primaryKey(),
    /** Whether the ticket is open. */
    status: text("status").notNull(),
});

/** A job waiting for a note its stored condition matches. */
const waiter = defineTable(
    "controller_waiter",
    {
        /** The waiter's name. */
        id: text("id").primaryKey(),
        /** The space with the waiter. */
        scope: text("scope").notNull(),
        /** The condition a note it waits for matches. */
        condition: json("condition", Condition.schema).notNull(),
    },
    { log: {} },
);

/** A challenge deleted once its expiry passes. */
const challenge = defineTable(
    "controller_challenge",
    {
        /** The challenge's name. */
        id: text("id").primaryKey(),
        /** When the challenge expires. */
        expiresAt: integer("expires_at").notNull(),
    },
    { expiry: "expiresAt" },
);

/** How many notes a test commits one at a time. */
const NOTE_COMMITS = 20;

/** Read a reported failure's message, refusing a failure that is not an error. */
function message(error: unknown): string {
    if (!(error instanceof Error)) {
        throw new TypeError("a reconciliation failed with a value that is not an error");
    }

    return error.message;
}

/** Run a world until a read equals the expected value, then assert it whole. */
async function waitEqual<Value>(simulation: Simulation, read: () => Value, expected: Value) {
    await simulation.waitUntil(() => canonicalize(read()) === canonicalize(expected));
    expect(read()).toEqual(expected);
}

/** Fail on a reported reconciliation failure. */
function failReconcile(
    _controller: Controller | Projection | Cursor,
    key: string,
    error: unknown,
): void {
    throw new Error(`reconciling '${key}' failed`, { cause: error });
}

/** Ignore a failed reconciliation. */
function ignore(): void {}

/** Open a world's database with jobs and locks, and a helper writing jobs. */
async function openJobs(
    simulation: Simulation,
    tables: readonly Table[] = [job, note, lock],
    telemetry?: SimulatedTelemetry,
) {
    const world = simulation.environment("test");
    const environment = telemetry === undefined ? world : { ...world, telemetry };
    const storage = await TestDatabase.create(tables, { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const write = (rows: readonly { id: string; status: string; dueAt: number }[]) =>
        database.transaction((transaction) =>
            transaction
                .insert(job)
                .values(rows.map((row) => ({ ...row, scope: "space" })))
                .execute(),
        );

    return { storage, database, write, world };
}

/** Run a loop until the test ends. */
function running(loop: ControlLoop): void {
    const stopping = new AbortController();
    const run = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await run;
    });
}

test("reconcile the pending rows listed at the start and those a change makes pending, leaving the rest alone, and a refused key until its next change", async () => {
    // record an open job, a done one and a refused one
    await using simulation = testSimulation();
    const recorded = new SimulatedTelemetry(simulation.environment("test"));
    using telemetry = new TestTelemetry(recorded);
    const { database, write } = await openJobs(simulation, [job, note, lock], recorded);
    await database.transaction((transaction) =>
        transaction
            .insert(job)
            .values([
                { id: "open", scope: "space", status: "open", dueAt: 0 },
                { id: "done", scope: "space", status: "done", dueAt: 0 },
            ])
            .execute(),
    );

    // close each reconciled job, refusing "refused" once
    const reconciled: string[] = [];
    const reported: string[] = [];
    let isRefusing = true;
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        reconcile: async (rows) => {
            const row = nth(rows, 0);
            reconciled.push(row.id);
            if (row.id === "refused" && isRefusing) {
                isRefusing = false;
                throw Failure.error(Failure.http("CONFLICT", "refused"));
            }
            await database.transaction((transaction) =>
                transaction.update(job).set({ status: "done" }).where(eq(job.id, row.id)).execute(),
            );
        },
    };
    running(
        new ControlLoop(database, [{ projections: [], controllers: [controller], cursors: [] }], {
            report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
        }),
    );

    // reconcile the listed job, then the written ones entering pending
    await waitEqual(simulation, () => reconciled, ["open"]);
    await write([
        { id: "later", status: "open", dueAt: 0 },
        { id: "refused", status: "open", dueAt: 0 },
    ]);
    await waitEqual(simulation, () => reconciled.toSorted(), ["later", "open", "refused"]);

    // reconcile the refused job again on its next change
    await database.transaction((transaction) =>
        transaction.update(job).set({ dueAt: 1 }).where(eq(job.id, "refused")).execute(),
    );
    await waitEqual(simulation, () => reconciled.toSorted(), [
        "later",
        "open",
        "refused",
        "refused",
    ]);

    // report the refusal once, recorded in db's scope
    expect([reported, telemetry.take()]).toEqual([
        ['{"id":"refused"}: refused'],
        [{ scope: "@destack/db", name: "Failure", message: "refused" }],
    ]);
});

test("report what made each pass's key due, and link the pass to the span of the transaction whose change did", async () => {
    // record a job open at the start
    await using simulation = testSimulation();
    const recorded = new SimulatedTelemetry(simulation.environment("test"));
    const { database, write } = await openJobs(simulation, [job, note, lock], recorded);
    await write([{ id: "listed", status: "open", dueAt: 0 }]);

    // close each reconciled job
    const reconciled: string[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        reconcile: async (rows) => {
            const row = nth(rows, 0);
            reconciled.push(row.id);
            await database.transaction((transaction) =>
                transaction.update(job).set({ status: "done" }).where(eq(job.id, row.id)).execute(),
            );
        },
    };
    running(
        new ControlLoop(database, [{ projections: [], controllers: [controller], cursors: [] }], {
            report: failReconcile,
        }),
    );

    // open another job once the listed one is reconciled
    await waitEqual(simulation, () => reconciled, ["listed"]);
    await write([{ id: "written", status: "open", dueAt: 0 }]);
    await waitEqual(simulation, () => reconciled, ["listed", "written"]);

    // run the listed job by the start's listing, and the written one by its change, linked to its transaction's span
    const names = new Map(recorded.spans.map((span) => [span.spanContext().spanId, span.name]));
    const passes = recorded.spans
        .filter((span) => span.name === "destack.controller.pass")
        .map((span) => [
            span.attributes["destack.key"],
            span.attributes["destack.wake.kind"],
            span.links.map((link) => [
                names.get(link.context.spanId),
                link.attributes?.["db.collection.name"],
            ]),
        ]);
    expect(passes).toEqual([
        ['{"id":"listed"}', "start", []],
        [
            '{"id":"written"}',
            "change",
            [["destack.transaction.run", "destack__db__controller_job"]],
        ],
    ]);
});

test("reconcile a waiting row once its time comes due, waking at the earliest one and setting the host's alarm there", async () => {
    // record jobs due later, one already due, and a done one never due
    const simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    const clock = simulation.clock;
    const started = clock.now();
    await write([
        { id: "due", status: "open", dueAt: started },
        { id: "soon", status: "open", dueAt: started + 50 },
        { id: "later", status: "open", dueAt: started + 80 },
    ]);

    // reconcile the open jobs due now, closing each
    const reconciled: [string, number][] = [];
    const alarms: (number | null)[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open", dueAt: { lte: now } },
        reconcile: async (rows) => {
            for (const row of rows) {
                reconciled.push([row.id, clock.now() - started]);
                await database.transaction((transaction) =>
                    transaction
                        .update(job)
                        .set({ status: "done" })
                        .where(eq(job.id, row.id))
                        .execute(),
                );
            }
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        {
            report: (_controller, _key, error) => {
                throw error;
            },
            alarm: {
                setAlarm: async (at) => {
                    alarms.push(at - started);
                },
                deleteAlarm: async () => {
                    alarms.push(null);
                },
            },
        },
    );
    const stopping = new AbortController();
    const run = loop.run(stopping.signal);

    // run the due job, then each later one at its time, and clear the alarm once none waits
    await loop.idle();
    await simulation.jump(started + 50);
    await loop.idle();
    await simulation.jump(started + 80);
    await loop.idle();
    stopping.abort();
    await run;
    expect({ reconciled, alarms }).toEqual({
        reconciled: [
            ["due", 0],
            ["soon", 50],
            ["later", 80],
        ],
        alarms: [0, 50, 80, null],
    });
});

test("reconcile every pending row under one key when a controller groups by no field, listed at the start and on a change", async () => {
    // record two open jobs before the start
    await using simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    await write([
        { id: "first", status: "open", dueAt: 0 },
        { id: "second", status: "open", dueAt: 0 },
    ]);

    // close every open job in one reconciliation
    const batches: string[][] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        key: [],
        reconcile: async (rows) => {
            batches.push(rows.map((row) => row.id).toSorted());
            await database.transaction((transaction) =>
                transaction
                    .update(job)
                    .set({ status: "done" })
                    .where(
                        inArray(
                            job.id,
                            rows.map((row) => row.id),
                        ),
                    )
                    .execute(),
            );
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: ignore },
    );
    running(loop);

    // take both at the start, then a later one alone
    await loop.idle();
    await write([{ id: "third", status: "open", dueAt: 0 }]);
    await waitEqual(simulation, () => batches, [["first", "second"], ["third"]]);
});

test("compare a microsecond field to the loop's time a window earlier, waking as each row leaves the window", async () => {
    // record jobs due in microseconds, one past the window and one inside it
    const simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    const clock = simulation.clock;
    const started = clock.now();
    await write([
        { id: "old", status: "open", dueAt: (started - 100) * 1000 },
        { id: "young", status: "open", dueAt: (started - 40) * 1000 + 1 },
    ]);

    // close each job once it is 60 milliseconds old
    const reconciled: [string, number][] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open", dueAt: { lte: now.microseconds().minus({ milliseconds: 60 }) } },
        reconcile: async (rows) => {
            for (const row of rows) {
                reconciled.push([row.id, clock.now() - started]);
                await database.transaction((transaction) =>
                    transaction
                        .update(job)
                        .set({ status: "done" })
                        .where(eq(job.id, row.id))
                        .execute(),
                );
            }
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: ignore },
    );
    running(loop);

    // close the old job at once, and the young one at the first millisecond past its window
    await loop.idle();
    await simulation.jump(started + 21);
    await loop.idle();
    expect(reconciled).toEqual([
        ["old", 0],
        ["young", 21],
    ]);
});

test("wake the jobs a relation reaches from a watched table's changed row", async () => {
    // record an open job in each of two spaces, and relate a note to its space's jobs
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation);
    await database.transaction((transaction) =>
        transaction
            .insert(job)
            .values([
                { id: "first", scope: "space", status: "open", dueAt: 0 },
                { id: "second", scope: "other", status: "open", dueAt: 0 },
            ])
            .execute(),
    );
    const related = defineRelations({ note, job }, (relate) => ({
        note: { jobs: relate.many.job({ from: relate.note.scope, to: relate.job.scope }) },
    }));

    // reconcile each job alone, woken through a note's relation to its space's jobs
    const reconciled: string[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        watches: [{ table: note, path: [related.get(note, "jobs")] }],
        reconcile: async (rows) => {
            reconciled.push(nth(rows, 0).id);
        },
    };
    const loop = new ControlLoop(
        database,
        [{ controllers: [controller], projections: [], cursors: [] }],
        { report: ignore },
    );
    running(loop);
    await loop.idle();
    const listed = reconciled.splice(0).toSorted();

    // reconcile the first again on a note in its space
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "note", scope: "space" }).execute(),
    );
    await loop.idle();
    expect({ listed, woken: reconciled }).toEqual({
        listed: ["first", "second"],
        woken: ["first"],
    });
});

test("wake only the jobs whose stored condition a watched table's changed row matches", async () => {
    // record two waiting jobs in one space, each waiting for a note of its own name
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [waiter, note, lock]);
    await database.transaction((transaction) =>
        transaction
            .insert(waiter)
            .values([
                { id: "first", scope: "space", condition: { id: "first" } },
                { id: "second", scope: "space", condition: { id: "second" } },
            ])
            .execute(),
    );

    // reconcile a waiter once a note matching its condition changes
    const reconciled: string[] = [];
    const controller: Controller<typeof waiter> = {
        name: "waiter",
        table: waiter,
        pending: { scope: "space" },
        watches: [{ table: note, matches: { column: "condition", where: { scope: "space" } } }],
        reconcile: async (rows) => {
            reconciled.push(nth(rows, 0).id);
        },
    };
    const loop = new ControlLoop(
        database,
        [{ controllers: [controller], projections: [], cursors: [] }],
        { report: ignore },
    );
    running(loop);
    await loop.idle();
    reconciled.splice(0);

    // reconcile the second alone on the note it waits for
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "second", scope: "space" }).execute(),
    );
    await loop.idle();
    expect(reconciled).toEqual(["second"]);
});

test("refuse a pending condition comparing now anywhere but a field's upper bound at its top", async () => {
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation);
    const declare = (pending: Controller<typeof job>["pending"]) => () =>
        new ControlLoop(
            database,
            [
                {
                    projections: [],
                    controllers: [{ name: "job", table: job, pending, reconcile: async () => {} }],
                    cursors: [],
                },
            ],
            { report: ignore },
        );

    expect(declare({ dueAt: { gte: now } })).toThrow(
        "controller 'job' compares now outside a field's lt or lte at the top of its pending condition or of its OR's branches",
    );
    expect(declare({ OR: [{ AND: [{ dueAt: { lte: now } }] }] })).toThrow(
        "controller 'job' compares now outside a field's lt or lte at the top of its pending condition or of its OR's branches",
    );
});

test("reconcile a row once the time in one of its pending condition's OR branches comes", async () => {
    // keep a done job due in a minute, pending while open or once due
    await using simulation = testSimulation();
    const { database, world, write } = await openJobs(simulation);
    const start = world.clock.now();
    await write([{ id: "later", status: "done", dueAt: start + 60_000 }]);
    const reconciled: number[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { OR: [{ status: "open" }, { dueAt: { lte: now } }] },
        reconcile: async (rows) => {
            reconciled.push(world.clock.now() - start);
            await database.transaction((transaction) =>
                transaction
                    .update(job)
                    .set({ dueAt: Number.MAX_SAFE_INTEGER })
                    .where(eq(job.id, nth(rows, 0).id))
                    .execute(),
            );
        },
    };
    const loop = new ControlLoop(
        database,
        [{ controllers: [controller], projections: [], cursors: [] }],
        { report: failReconcile },
    );
    const stopping = new AbortController();
    const run = loop.run(stopping.signal);

    // reconcile it at its due time, not before
    await loop.idle();
    const before = [...reconciled];
    await simulation.jump(start + 60_000);
    await loop.idle();
    stopping.abort();
    await run;
    expect({ before, after: reconciled }).toEqual({ before: [], after: [60_000] });
});

test("select the jobs a watched table's change names", async () => {
    // record an open job in each of two spaces
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation);
    await database.transaction((transaction) =>
        transaction
            .insert(job)
            .values([
                { id: "first", scope: "space", status: "open", dueAt: 0 },
                { id: "second", scope: "other", status: "open", dueAt: 0 },
            ])
            .execute(),
    );

    // reconcile each space's jobs together, selected by a note in the space
    const reconciled: string[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        key: ["scope"],
        watches: [{ table: note, on: { scope: "scope" } }],
        reconcile: async (rows) => {
            reconciled.push(rows.map((row) => row.id).join(","));
        },
    };
    const loop = new ControlLoop(
        database,
        [{ controllers: [controller], projections: [], cursors: [] }],
        { report: ignore },
    );
    running(loop);
    await loop.idle();

    // reconcile a space's jobs again on a note in the space
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "note", scope: "space" }).execute(),
    );
    await loop.idle();
    expect(reconciled.toSorted()).toEqual(["first", "first", "second"]);
});

test("wake a watch on an observation only when it observes, and on a desired change always", async () => {
    // record a job of a space, and an order there whose watches select it
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [job, note, order, lock]);
    await database.transaction((transaction) =>
        transaction
            .insert(job)
            .values({ id: "first", scope: "space", status: "open", dueAt: 0 })
            .execute(),
    );
    await database.transaction((transaction) =>
        transaction
            .insert(order)
            .values({ id: "order", scope: "space", generation: 1, state: "new" })
            .execute(),
    );

    // count the passes of a controller watching desired changes, and of one observing too
    const counted = { desired: 0, observing: 0 };
    const controller = (
        name: keyof typeof counted,
        isObserving: boolean,
    ): Controller<typeof job> => ({
        name,
        table: job,
        pending: { status: "open" },
        key: ["scope"],
        watches: [
            {
                table: order,
                isObserving,
                on: { scope: "scope" },
            },
        ],
        reconcile: async () => {
            counted[name] += 1;
        },
    });
    const loop = new ControlLoop(
        database,
        [
            {
                projections: [],
                controllers: [controller("desired", false), controller("observing", true)],
                cursors: [],
            },
        ],
        { report: ignore },
    );
    running(loop);
    await loop.idle();
    const started = { ...counted };

    // update the order's state alone, then its desired state
    await database.transaction((transaction) =>
        transaction.update(order).set({ state: "running" }).where(eq(order.id, "order")).execute(),
    );
    await loop.idle();
    const noted = { ...counted };
    await database.transaction((transaction) =>
        transaction
            .update(order)
            .set({ state: "new", generation: 2 })
            .where(eq(order.id, "order"))
            .execute(),
    );
    await loop.idle();

    // wake the observing watch alone on the observation, and both on the desired change
    expect({
        observation: [noted.desired - started.desired, noted.observing - started.observing],
        desired: [counted.desired - noted.desired, counted.observing - noted.observing],
    }).toEqual({ observation: [0, 1], desired: [1, 1] });
});

test("report a key failing transiently only once its retries last past the window, and any other failure at once", async () => {
    // record three open jobs
    const simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    await write([
        { id: "peer", status: "open", dueAt: 0 },
        { id: "broken", status: "open", dueAt: 0 },
        { id: "misrouted", status: "open", dueAt: 0 },
    ]);

    // leave "peer" unavailable until told otherwise, fail "broken" once, and misroute "misrouted" once
    const clock = simulation.clock;
    const started = clock.now();
    const reported: [string, number][] = [];
    let isDown = true;
    let isBroken = true;
    let isMisrouted = true;
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        reconcile: async (rows) => {
            const { id } = nth(rows, 0);
            if (id === "peer" && isDown) {
                throw Failure.error(Failure.http("SERVICE_UNAVAILABLE", "service unavailable"));
            } else if (id === "broken" && isBroken) {
                isBroken = false;
                throw Failure.error(Failure.http("CONFLICT", "broken"));
            } else if (id === "misrouted" && isMisrouted) {
                isMisrouted = false;
                throw Failure.error({
                    code: "MISDIRECTED_REQUEST",
                    reason: "PARTITION_MOVED",
                    domain: "@example/space",
                    message: "space lives in another partition",
                });
            }
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        {
            report: (_controller, key) => reported.push([key, clock.now() - started]),
            retry: { initialInterval: 5, maximumInterval: 5 },
            transientFor: 40,
        },
    );
    running(loop);

    // report "broken" at once and retry the others, then report "peer" once its retries fail past the window
    await loop.idle();
    const atOnce = [...reported];
    for (let retried = 0; retried < 8; retried += 1) {
        await simulation.jump(clock.now() + 5);
        await loop.idle();
    }
    isDown = false;
    await simulation.jump(clock.now() + 5);
    await loop.idle();
    expect({ atOnce, reported }).toEqual({
        atOnce: [['{"id":"broken"}', 0]],
        reported: [
            ['{"id":"broken"}', 0],
            ['{"id":"peer"}', 40],
        ],
    });
});

test("fail the loop, its run and its idle waiters, once a controller throws a failure it declares no outcome for", async () => {
    // reconcile one open job, its controller throwing a TypeError
    await using simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    await write([{ id: "job", status: "open", dueAt: 0 }]);
    const defect = new TypeError("undefined is not an object");
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        reconcile: async () => {
            throw defect;
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: ignore },
    );
    const run = loop.run(new AbortController().signal);

    // fail the run and the idle waiter with the controller's failure
    const [ran, idled] = await Promise.all([refusal(run), refusal(loop.idle())]);
    expect([ran, idled]).toEqual([
        [
            "INTERNAL_SERVER_ERROR",
            "CONTROLLER_BROKEN",
            "'job' did not reconcile '{\"id\":\"job\"}'",
        ],
        [
            "INTERNAL_SERVER_ERROR",
            "CONTROLLER_BROKEN",
            "'job' did not reconcile '{\"id\":\"job\"}'",
        ],
    ]);
});

test("reconcile keys up to a controller's concurrency, never one key twice at once", async () => {
    // record three open jobs
    await using simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    await write(["a", "b", "c"].map((id) => ({ id, status: "open", dueAt: 0 })));

    // block each reconciliation until released
    const releases = new Map<string, () => void>();
    let active = 0;
    let most = 0;
    const started: string[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        concurrency: 2,
        reconcile: async (rows) => {
            const { id } = nth(rows, 0);
            started.push(id);
            active += 1;
            most = Math.max(most, active);
            await new Promise<void>((release) => {
                releases.set(id, release);
            });
            active -= 1;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: ignore },
    );
    const looping = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        releases.forEach((release) => release());
        await looping;
    });

    // change the first while it runs and finish the second
    await waitEqual(simulation, () => started.length, 2);
    const first = nth(started, 0);
    const second = nth(started, 1);
    const third = nth(
        ["a", "b", "c"].filter((id) => !started.includes(id)),
        0,
    );
    await database.transaction((transaction) =>
        transaction.update(job).set({ dueAt: 1 }).where(eq(job.id, first)).execute(),
    );
    found(releases, second)();
    await waitEqual(simulation, () => started, [first, second, third]);

    // reconcile the first again after it finishes
    found(releases, first)();
    await waitEqual(simulation, () => started, [first, second, third, first]);
    expect(most).toBe(2);
});

test("lock each key to one instance of several sharing a database, handing it over once its holder stops", async () => {
    // run two instances of the loop over two connections to one database
    await using simulation = testSimulation();
    const { storage, database, write } = await openJobs(simulation);
    const other = await storage.connect([job, note, lock]);
    onTestFinished(() => other.close());
    await write([{ id: "shared", status: "open", dueAt: 0 }]);

    // record which instance reconciles the shared job, holding it until stopped
    const reconciled: string[] = [];
    const controllerOf = (instance: string): Controller<typeof job> => ({
        name: "job",
        table: job,
        pending: { status: "open" },
        reconcile: async (_rows, { signal }) => {
            reconciled.push(instance);
            await waitAborted(signal);
        },
    });
    const first = new AbortController();
    const second = new AbortController();
    const runs = [
        new ControlLoop(
            database,
            [{ projections: [], controllers: [controllerOf("first")], cursors: [] }],
            { report: ignore },
        ).run(first.signal),
        new ControlLoop(
            other,
            [{ projections: [], controllers: [controllerOf("second")], cursors: [] }],
            { report: ignore },
        ).run(second.signal),
    ];
    onTestFinished(async () => {
        first.abort();
        second.abort();
        await Promise.all(runs);
    });

    // reconcile the job on one instance alone, then hand it to the other once the holder stops
    await waitEqual(simulation, () => reconciled.length, 1);
    const holder = nth(reconciled, 0);
    (holder === "first" ? first : second).abort();
    await nth(runs, holder === "first" ? 0 : 1);
    await expect.poll(() => reconciled).toEqual([holder, holder === "first" ? "second" : "first"]);
});

test("follow a key while its rows stay pending, stopping it once they leave or the loop stops", async () => {
    // record two open jobs
    await using simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    await write([
        { id: "kept", status: "open", dueAt: 0 },
        { id: "closed", status: "open", dueAt: 0 },
    ]);

    // follow each open job until stopped
    const events: string[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        concurrency: Infinity,
        follow: async (rows, { signal, synced }) => {
            const { id } = nth(rows, 0);
            events.push(`${id} started`);
            synced();
            await waitAborted(signal);
            events.push(`${id} stopped`);
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: ignore },
    );
    const run = loop.run(stopping.signal);
    await loop.idle();

    // stop the closed job's follow, then every follow with the loop
    await database.transaction((transaction) =>
        transaction.update(job).set({ status: "done" }).where(eq(job.id, "closed")).execute(),
    );
    await waitEqual(simulation, () => events.includes("closed stopped"), true);
    stopping.abort();
    await run;
    expect(events.toSorted()).toEqual([
        "closed started",
        "closed stopped",
        "kept started",
        "kept stopped",
    ]);
});

test("report a follow that fails while its key stays pending", async () => {
    // record an open job
    await using simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    await write([{ id: "refused", status: "open", dueAt: 0 }]);

    // fail the follow at once with a refusal
    const reported: string[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        follow: async () => {
            throw Failure.error(Failure.http("FORBIDDEN", "refused"));
        },
    };
    running(
        new ControlLoop(database, [{ projections: [], controllers: [controller], cursors: [] }], {
            report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
        }),
    );

    // report the refusal for the key
    await waitEqual(simulation, () => reported, ['{"id":"refused"}: refused']);
});

test("take a cursor's changes past its checkpoint, those before the start and those committed later, each once", async () => {
    // append a note in space a and one in b before the start
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [note, lock]);
    const append = (id: string, scope: string) =>
        database.transaction((transaction) =>
            transaction.insert(note).values({ id, scope }).execute(),
        );
    await append("first", "a");
    await append("other", "b");

    // take the notes past the checkpoint and move it past them
    const taken: string[] = [];
    const cursor: Cursor = {
        name: "note-cursor",
        tables: [note],
        take: async () => {
            const after = (await database.log.checkpoint("note-cursor")) ?? 0;
            const page = await database.log.read({ tables: [note], after });
            taken.push(...page.changes.map((change) => `${change.scope}/${change.key["id"]}`));
            await database.log.move(
                { name: "note-cursor", tables: [note] },
                after,
                page.sequence,
                Number.MAX_SAFE_INTEGER,
            );
        },
    };
    const reported: string[] = [];
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [], cursors: [cursor] }],
        {
            report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
        },
    );
    running(loop);

    // take the notes appended before the start, then those appended later
    await waitEqual(simulation, () => taken, ["a/first", "b/other"]);
    await append("second", "a");
    await append("again", "b");
    await waitEqual(simulation, () => taken, ["a/first", "b/other", "a/second", "b/again"]);
    await loop.idle();
    expect([taken, reported]).toEqual([["a/first", "b/other", "a/second", "b/again"], []]);
});

test("take a cursor's changes once its first untaken one is a batch old, at the start and as they commit", async () => {
    // place the cursor and append a note before the start
    const simulation = testSimulation();
    const { database } = await openJobs(simulation, [note, lock]);
    const started = simulation.clock.now();
    await database.log.place({ name: "note-batch", tables: [note] }, Number.MAX_SAFE_INTEGER);
    const append = (id: string) =>
        database.transaction((transaction) =>
            transaction.insert(note).values({ id, scope: "a" }).execute(),
        );
    await append("first");

    // record when each page is taken, moving the checkpoint past it
    const taken: number[] = [];
    const cursor: Cursor = {
        name: "note-batch",
        tables: [note],
        batch: 50,
        take: async () => {
            const after = (await database.log.checkpoint("note-batch")) ?? 0;
            const page = await database.log.read({ tables: [note], after });
            taken.push(simulation.clock.now() - started);
            await database.log.move(
                { name: "note-batch", tables: [note] },
                after,
                page.sequence,
                Number.MAX_SAFE_INTEGER,
            );
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [], cursors: [cursor] }],
        { report: ignore },
    );
    const stopping = new AbortController();
    const run = loop.run(stopping.signal);

    // take the first a batch after it, then a later append a batch after it
    await loop.idle();
    await simulation.jump(started + 50);
    await loop.idle();
    await simulation.jump(started + 60);
    await append("second");
    await loop.idle();
    await simulation.jump(started + 110);
    await loop.idle();
    stopping.abort();
    await run;
    expect(taken).toEqual([50, 110]);
});

test("project every key at the first start, then only the keys the changes past the loop's position touch, across restarts", async () => {
    // record notes in two spaces before the first start
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [note, lock]);
    const append = (id: string, scope: string) =>
        database.transaction((transaction) =>
            transaction.insert(note).values({ id, scope }).execute(),
        );
    await append("first", "a");
    await append("second", "b");

    // project a space from its notes, listing every space with notes
    const projected: string[] = [];
    const projection: Projection = {
        name: "note-spaces",
        key: "scope",
        watches: [{ table: note, on: { scope: "scope" } }],
        all: { table: note, column: "scope" },
        result: "rows",
        project: async (key) => {
            projected.push(key);
        },
    };
    const start = async () => {
        const stopping = new AbortController();
        const loop = new ControlLoop(
            database,
            [{ controllers: [], projections: [projection], cursors: [] }],
            { report: ignore, position: "control" },
        );
        const run = loop.run(stopping.signal);
        await loop.idle();

        return async () => {
            stopping.abort();
            await run;
        };
    };

    // project both at the first start, which places the position, then the one a note touches
    const stop = await start();
    const initial = projected.splice(0);
    const isPlaced = (await database.log.checkpoint("control")) !== undefined;
    await append("third", "b");
    await simulation.waitUntil(() => projected.length > 0);
    const changed = projected.splice(0);
    await stop();

    // project nothing at an unchanged restart, and the space a note touched while stopped at the next
    const unchanged = await start();
    const restarted = projected.splice(0);
    await unchanged();
    await append("fourth", "a");
    const caught = await start();
    const resumed = projected.splice(0);
    await caught();
    expect({ initial: initial.toSorted(), isPlaced, changed, restarted, resumed }).toEqual({
        initial: ["a", "b"],
        isPlaced: true,
        changed: ["b"],
        restarted: [],
        resumed: ["a"],
    });
});

test("project a key again that was running when the loop stopped", async () => {
    // record a note, and project its space until the loop stops the first time
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [note, lock]);
    const append = (id: string, scope: string) =>
        database.transaction((transaction) =>
            transaction.insert(note).values({ id, scope }).execute(),
        );
    await append("first", "a");
    const projected: string[] = [];
    let isHeld = false;
    const projection: Projection = {
        name: "note-spaces",
        key: "scope",
        watches: [{ table: note, on: { scope: "scope" } }],
        all: { table: note, column: "scope" },
        result: "rows",
        project: async (key, reconciliation) => {
            projected.push(key);
            if (isHeld) {
                await waitAborted(reconciliation.signal);
            }
        },
    };
    const start = () => {
        const stopping = new AbortController();
        const loop = new ControlLoop(
            database,
            [{ controllers: [], projections: [projection], cursors: [] }],
            { report: ignore, position: "control" },
        );
        const run = loop.run(stopping.signal);

        return {
            loop,
            stop: async () => {
                stopping.abort();
                await run;
            },
        };
    };

    // place the position, then stop while a changed space still projects
    const first = start();
    await first.loop.idle();
    projected.splice(0);
    isHeld = true;
    await append("second", "b");
    await simulation.waitUntil(() => projected.length > 0);
    const held = projected.splice(0);
    await first.stop();

    // project the space again at the next start, as its pass never finished
    isHeld = false;
    const second = start();
    await second.loop.idle();
    const resumed = projected.splice(0);
    await second.stop();
    expect({ held, resumed }).toEqual({ held: ["b"], resumed: ["b"] });
});

test("list a projection kept in the process at each start, beside one continuing from the position", async () => {
    // record a note, and project its space into rows and into the process
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [note, lock]);
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "first", scope: "a" }).execute(),
    );
    const projected: string[] = [];
    const projectionOf = (name: string, result: Projection["result"]): Projection => ({
        name,
        key: "scope",
        watches: [{ table: note, on: { scope: "scope" } }],
        all: { table: note, column: "scope" },
        result,
        project: async (key) => {
            projected.push(`${name} ${key}`);
        },
    });
    const start = async () => {
        const stopping = new AbortController();
        const projections = [projectionOf("rows", "rows"), projectionOf("process", "process")];
        const loop = new ControlLoop(database, [{ controllers: [], projections, cursors: [] }], {
            report: ignore,
            position: "control",
        });
        const run = loop.run(stopping.signal);
        await loop.idle();
        stopping.abort();
        await run;

        // read which projections ran, as a key may project again for a change the listing already saw
        return [...new Set(projected.splice(0))].toSorted();
    };

    // project both at the first start, and the one kept in the process alone at an unchanged restart
    const initial = await start();
    const restarted = await start();
    expect({ initial, restarted }).toEqual({
        initial: ["process a", "rows a"],
        restarted: ["process a"],
    });
});

test("replay a late set's projection from the position", async () => {
    // place the position
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [note, lock]);
    const append = (id: string, scope: string) =>
        database.transaction((transaction) =>
            transaction.insert(note).values({ id, scope }).execute(),
        );
    await append("first", "a");
    const projected: string[] = [];
    const projection: Projection = {
        name: "note-spaces",
        key: "scope",
        watches: [{ table: note, on: { scope: "scope" } }],
        all: { table: note, column: "scope" },
        result: "rows",
        project: async (key) => {
            projected.push(key);
        },
    };
    const start = async (activation?: Promise<void>) => {
        const stopping = new AbortController();
        const set = { controllers: [], projections: [projection], cursors: [] };
        const loop = new ControlLoop(
            database,
            [activation === undefined ? set : { ...set, activation: () => activation }],
            { report: ignore, position: "control" },
        );
        const run = loop.run(stopping.signal);
        await loop.idle();

        return {
            loop,
            stop: async () => {
                stopping.abort();
                await run;
            },
        };
    };
    await (await start()).stop();
    projected.splice(0);

    // note a space, then start inactive
    await append("second", "b");
    const activated = Promise.withResolvers<void>();
    const waiting = await start(activated.promise);
    const before = projected.splice(0);

    // activate
    activated.resolve();
    await simulation.waitUntil(() => projected.length > 0);
    await waiting.loop.idle();
    const after = projected.splice(0);
    await waiting.stop();
    expect({ before, after }).toEqual({ before: [], after: ["b"] });
});

test("wake a row at its earliest OR branch's time, a branch with a missing time leaving it", async () => {
    // keep jobs pending once done and due, or due a moment later when early
    await using simulation = testSimulation();
    const { database, world } = await openJobs(simulation, [ticket, job, lock]);
    const start = world.clock.now();
    const reconciled: [string, number][] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: {
            OR: [
                { dueAt: { lte: now }, status: "done" },
                { dueAt: { lt: now }, id: "early" },
            ],
        },
        reconcile: async (rows) => {
            const { id } = nth(rows, 0);
            reconciled.push([id, world.clock.now() - start]);
            await database.transaction((transaction) =>
                transaction
                    .update(job)
                    .set({ dueAt: Number.MAX_SAFE_INTEGER })
                    .where(eq(job.id, id))
                    .execute(),
            );
        },
    };
    const loop = new ControlLoop(
        database,
        [{ controllers: [controller], projections: [], cursors: [] }],
        { report: failReconcile },
    );
    running(loop);

    // add the jobs once the loop runs, so each change sets its own wake
    await loop.idle();
    await database.transaction((transaction) =>
        transaction
            .insert(job)
            .values([
                { id: "early", scope: "space", status: "done", dueAt: start + 60_000 },
                { id: "late", scope: "space", status: "done", dueAt: start + 120_000 },
            ])
            .execute(),
    );

    // reconcile each at its own time
    await loop.idle();
    await simulation.jump(start + 60_000);
    await loop.idle();
    await simulation.jump(start + 120_000);
    await loop.idle();
    expect(reconciled).toEqual([
        ["early", 60_000],
        ["late", 120_000],
    ]);
});

test("delete a table's rows once their expiry passes", async () => {
    // store a challenge expiring now and one expiring in a minute
    await using simulation = testSimulation();
    const { database, world } = await openJobs(simulation, [challenge, lock]);
    const start = world.clock.now();
    await database.transaction((transaction) =>
        transaction
            .insert(challenge)
            .values([
                { id: "lapsed", expiresAt: start },
                { id: "fresh", expiresAt: start + 60_000 },
            ])
            .execute(),
    );
    const loop = new ControlLoop(
        database,
        [{ controllers: [ControlLoop.expiry(challenge, database)], projections: [], cursors: [] }],
        { report: failReconcile },
    );
    const stopping = new AbortController();
    const run = loop.run(stopping.signal);
    const read = async () =>
        (await database.select().from(challenge)).map((row) => row.id).toSorted();

    // delete the lapsed one at once, and the fresh one a minute later
    await loop.idle();
    const first = await read();
    await simulation.jump(start + 60_000);
    await loop.idle();
    const second = await read();
    stopping.abort();
    await run;
    expect({ first, second }).toEqual({ first: ["fresh"], second: [] });
});

test("list a controller of an unlogged table again after a commit to its table", async () => {
    // reconcile open tickets of an unlogged table
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [ticket, lock]);
    const reconciled: string[] = [];
    const controller: Controller<typeof ticket> = {
        name: "ticket",
        table: ticket,
        pending: { status: "open" },
        reconcile: async (rows) => {
            const { id } = nth(rows, 0);
            reconciled.push(id);
            await database.update(ticket).set({ status: "done" }).where(eq(ticket.id, id));
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: ignore },
    );
    running(loop);
    await loop.idle();

    // reconcile a ticket written after the start
    await database.insert(ticket).values({ id: "first", status: "open" });
    await waitEqual(simulation, () => reconciled, ["first"]);
});

test("take a cursor once more after a commit to an unlogged table it reads, then go idle", async () => {
    // count the takes of a cursor over notes and unlogged tickets
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [note, ticket, lock]);
    let takes = 0;
    const cursor: Cursor = {
        name: "note-cursor",
        tables: [note, ticket],
        take: async () => {
            takes += 1;
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [], cursors: [cursor] }],
        { report: ignore },
    );
    running(loop);
    await loop.idle();
    const before = takes;

    // write a ticket
    await database.insert(ticket).values({ id: "first", status: "open" });
    await waitEqual(simulation, () => takes, before + 1);
    await loop.idle();
    expect(takes).toBe(before + 1);
});

test("project every key again once the changes past the loop's position were compacted while stopped", async () => {
    // project the spaces with notes, resuming from the loop's position
    await using simulation = testSimulation();
    const { database, world } = await openJobs(simulation, [note, lock]);
    const append = (id: string, scope: string) =>
        database.transaction((transaction) =>
            transaction.insert(note).values({ id, scope }).execute(),
        );
    await append("first", "a");
    const projected: string[] = [];
    const projection: Projection = {
        name: "note-spaces",
        key: "scope",
        watches: [{ table: note, on: { scope: "scope" } }],
        all: { table: note, column: "scope" },
        result: "rows",
        project: async (key) => {
            projected.push(key);
        },
    };
    const start = async () => {
        const stopping = new AbortController();
        const loop = new ControlLoop(
            database,
            [{ controllers: [], projections: [projection], cursors: [] }],
            { report: ignore, position: "control" },
        );
        const run = loop.run(stopping.signal);
        await loop.idle();

        return async () => {
            stopping.abort();
            await run;
        };
    };
    await (
        await start()
    )();
    projected.splice(0);

    // note a second space while stopped
    await append("second", "b");

    // compact the log past the position's expired hold
    const expired = world.clock.now() + 8 * 24 * 60 * 60_000;
    await database.log.compact(expired, expired);

    // project both again
    const stop = await start();
    await stop();
    expect(projected.toSorted()).toEqual(["a", "b"]);
});

test("select a reloaded table's changes by its name, reading pending rows through a table defined anew", async () => {
    // reconcile open jobs read through a table a module loaded again defined
    await using simulation = testSimulation();
    const { database, write } = await openJobs(simulation);
    const reconciled: string[] = [];
    const controller: Controller<typeof reloadedJob> = {
        name: "job",
        table: reloadedJob,
        pending: { status: "open" },
        reconcile: async (rows) => {
            reconciled.push(nth(rows, 0).id);
        },
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: ignore },
    );
    running(loop);
    await loop.idle();

    // reconcile a job written through the first definition
    await write([{ id: "first", status: "open", dueAt: 0 }]);
    await waitEqual(simulation, () => reconciled, ["first"]);
});

test("spend no statement on commits to tables no controller reads", async () => {
    // reconcile open jobs, and commit notes no controller reads
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation);
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        reconcile: async () => {},
    };
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: ignore },
    );
    running(loop);
    await loop.idle();

    // count each note commit's transaction and its insert alone, past the log's first compaction
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "first", scope: "space" }).execute(),
    );
    await loop.idle();
    const before = await Recording.read(simulation.telemetry);
    for (let index = 0; index < NOTE_COMMITS; index++) {
        await database.transaction((transaction) =>
            transaction
                .insert(note)
                .values({ id: `note-${index}`, scope: "space" })
                .execute(),
        );
        await yieldToEventLoop();
    }
    const statements = (await Recording.read(simulation.telemetry))
        .since(before)
        .count(TestDatabase.statements(database));
    expect(statements).toBe(2 * NOTE_COMMITS);
});

test("run timers between the reconciliations of a synchronous database, one key per turn", async () => {
    const { clock } = testHost();
    const storage = await TestDatabase.create([queuedJob, lock], {
        environment: testHost(),
        dialect: "sqlite",
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(queuedJob).values(
        Array.from({ length: BURST_KEYS }, (_, index) => ({
            id: `job-${index}`,
            status: "open",
        })),
    );

    // record the reconciliations and the probe's ticks in the order they run
    const events: ("reconcile" | "tick")[] = [];
    const done = Promise.withResolvers<void>();
    const controller: Controller<typeof queuedJob> = {
        name: "queued-job",
        table: queuedJob,
        pending: { status: "open" },
        reconcile: async (rows) => {
            // read the job a few times and work on it without yielding
            const [row] = rows;
            for (let read = 0; read < BURST_READS; read++) {
                await database
                    .select()
                    .from(queuedJob)
                    .where(eq(queuedJob.id, row?.id ?? ""));
            }
            const end = clock.now() + BURST_WORK_MILLISECONDS;
            while (clock.now() < end) {
                // occupy the thread as decoding does
            }

            // finish the burst at the last key
            events.push("reconcile");
            if (events.filter((event) => event === "reconcile").length === BURST_KEYS) {
                done.resolve();
            }
        },
    };

    // probe the event loop while the loop reconciles every key
    const probing = new AbortController();
    const probe = (async () => {
        // tick once per wait until the probe stops
        while (
            await clock.sleep(PROBE_MILLISECONDS, probing.signal).then(
                () => true,
                () => false,
            )
        ) {
            events.push("tick");
        }
    })();
    const stopping = new AbortController();
    const loop = new ControlLoop(
        database,
        [{ projections: [], controllers: [controller], cursors: [] }],
        { report: failReconcile },
    );
    const looping = loop.run(stopping.signal);
    await done.promise;
    probing.abort();
    await probe;
    stopping.abort();
    await looping;

    // tick between reconciliations after the first two, which share the turn the listing woke
    const reconciled = events.flatMap((event, index) => (event === "reconcile" ? [index] : []));
    const interleaved = reconciled
        .slice(1)
        .filter((at, index) => events.slice(reconciled[index], at).includes("tick"));
    expect([reconciled.length, interleaved.length]).toEqual([BURST_KEYS, BURST_KEYS - 2]);
});
