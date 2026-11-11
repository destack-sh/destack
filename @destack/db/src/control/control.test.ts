import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Change, defineTable, eq, inArray, integer, lock, type Table, text } from "../index.ts";
import { TestDatabase } from "../test/index.ts";
import { testSimulation } from "@destack/test/world";
import { canonicalize, nth, found, schema } from "@destack/schema";
import { ControlLoop, type Controller, now, type Cursor } from "./control.ts";
import { Projection } from "./projection.ts";
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
        new ControlLoop(
            database,
            { controllers: [controller], cursors: [] },
            {
                report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
            },
        ),
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
        { controllers: [controller], cursors: [] },
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
        { controllers: [controller], cursors: [] },
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
        { controllers: [controller], cursors: [] },
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

test("refuse a pending condition comparing now anywhere but a field's upper bound at its top", async () => {
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation);
    const declare = (pending: Controller<typeof job>["pending"]) => () =>
        new ControlLoop(
            database,
            {
                controllers: [{ name: "job", table: job, pending, reconcile: async () => {} }],
                cursors: [],
            },
            { report: ignore },
        );

    expect(declare({ dueAt: { gte: now } })).toThrow(
        "controller 'job' compares now outside a field's lt or lte at the top of its pending condition",
    );
    expect(declare({ OR: [{ dueAt: { lte: now } }] })).toThrow(
        "controller 'job' compares now outside a field's lt or lte at the top of its pending condition",
    );
});

test("select the jobs a watched table's change names, reporting a failing watch and listing in its place", async () => {
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
    const reported: string[] = [];
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        key: ["scope"],
        watches: [
            {
                table: note,
                keys: (change) => {
                    const scope = schema.string().parse(Change.image(change)["scope"]);
                    if (scope === "broken") {
                        throw new Error("unreadable note");
                    }

                    return [{ scope }];
                },
            },
        ],
        reconcile: async (rows) => {
            reconciled.push(rows.map((row) => row.id).join(","));
        },
    };
    const loop = new ControlLoop(
        database,
        { controllers: [controller], cursors: [] },
        {
            report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
        },
    );
    running(loop);
    await loop.idle();

    // reconcile a space's jobs again on a note, and list every space in place of a failing watch
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "note", scope: "space" }).execute(),
    );
    await loop.idle();
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "broken", scope: "broken" }).execute(),
    );
    await loop.idle();
    expect({ reconciled: reconciled.toSorted(), reported }).toEqual({
        reconciled: ["first", "first", "first", "second", "second"],
        reported: ["controller_note: unreadable note"],
    });
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
                keys: (change) => [{ scope: schema.string().parse(Change.image(change)["scope"]) }],
            },
        ],
        reconcile: async () => {
            counted[name] += 1;
        },
    });
    const loop = new ControlLoop(
        database,
        { controllers: [controller("desired", false), controller("observing", true)], cursors: [] },
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
        { controllers: [controller], cursors: [] },
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
        { controllers: [controller], cursors: [] },
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
        { controllers: [controller], cursors: [] },
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
            { controllers: [controllerOf("first")], cursors: [] },
            { report: ignore },
        ).run(first.signal),
        new ControlLoop(
            other,
            { controllers: [controllerOf("second")], cursors: [] },
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
    await expect
        .poll(() => reconciled, { timeout: 2000 })
        .toEqual([holder, holder === "first" ? "second" : "first"]);
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
        { controllers: [controller], cursors: [] },
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
        new ControlLoop(
            database,
            { controllers: [controller], cursors: [] },
            { report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`) },
        ),
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
        { controllers: [], cursors: [cursor] },
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
        { controllers: [], cursors: [cursor] },
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

test("project the keys of changes past one log page in one take, through the log's head", async () => {
    // place the projection's checkpoint, then record more notes than one page reads
    await using simulation = testSimulation();
    const { database } = await openJobs(simulation, [note, lock]);
    await database.log.place({ name: "note-pages", tables: [note] }, Number.MAX_SAFE_INTEGER);
    const scopes = Array.from({ length: 1100 }, (_, index) => `space-${index}`);
    for (let first = 0; first < scopes.length; first += 100) {
        const rows = scopes.slice(first, first + 100).map((scope) => ({ id: scope, scope }));
        await database.transaction((transaction) =>
            transaction.insert(note).values(rows).execute(),
        );
    }

    // project each space a note touches
    const projected: string[] = [];
    const projection = new Projection(database, {
        name: "note-pages",
        tables: [note],
        keys: async (change) => [schema.string().parse(Change.image(change)["scope"])],
        all: async () => [],
        project: async (key) => {
            projected.push(key);
        },
    });

    // take once, projecting every space, then settle, moving the checkpoint to the head
    await projection.take({ signal: new AbortController().signal, changed: async () => {} });
    await projection.settle();
    const { sequence } = await database.log.position();
    expect({
        projected: projected.toSorted(),
        checkpoint: await database.log.checkpoint("note-pages"),
    }).toEqual({ projected: scopes.toSorted(), checkpoint: sequence });
});

test("project every key at the first start, then only the keys the changes past its checkpoint touch, across restarts", async () => {
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
    const projection = () =>
        new Projection(database, {
            name: "note-spaces",
            tables: [note],
            keys: async (change) => [schema.string().parse(Change.image(change)["scope"])],
            all: async (connection) =>
                (await connection.selectDistinct({ scope: note.scope }).from(note)).map(
                    (row) => row.scope,
                ),
            project: async (key) => {
                projected.push(key);
            },
        });
    const start = async () => {
        const stopping = new AbortController();
        const loop = new ControlLoop(
            database,
            { controllers: [], cursors: [projection()] },
            { report: ignore },
        );
        const run = loop.run(stopping.signal);
        await loop.idle();

        return async () => {
            stopping.abort();
            await run;
        };
    };

    // project both at the first start, then the one a note touches
    const stop = await start();
    const initial = projected.splice(0);
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
    expect({ initial: initial.toSorted(), changed, restarted, resumed }).toEqual({
        initial: ["a", "b"],
        changed: ["b"],
        restarted: [],
        resumed: ["a"],
    });
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
        { controllers: [controller], cursors: [] },
        { report: ignore },
    );
    running(loop);
    await loop.idle();

    // reconcile a ticket written after the start
    await database.insert(ticket).values({ id: "first", status: "open" });
    await waitEqual(simulation, () => reconciled, ["first"]);
});

test("list again after the follow's changes were compacted, reconciling the rows pending meanwhile", async () => {
    // reconcile open jobs, holding the first until its follow's later changes are compacted
    await using simulation = testSimulation();
    const { database, write, world } = await openJobs(simulation);
    const reconciled: string[] = [];
    const held = Promise.withResolvers<void>();
    const release = Promise.withResolvers<void>();
    const controller: Controller<typeof job> = {
        name: "job",
        table: job,
        pending: { status: "open" },
        watches: [
            {
                table: note,
                keys: async () => {
                    held.resolve();
                    await release.promise;

                    return [];
                },
            },
        ],
        reconcile: async (rows) => {
            reconciled.push(nth(rows, 0).id);
        },
    };
    const loop = new ControlLoop(
        database,
        { controllers: [controller], cursors: [] },
        { report: ignore },
    );
    running(loop);
    await loop.idle();

    // stop the follow at a note, write a job, and compact both
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "note", scope: "space" }).execute(),
    );
    await held.promise;
    await write([{ id: "meanwhile", status: "open", dueAt: 0 }]);
    const at = world.clock.now();
    await database.log.compact(at + 1, at);

    // list again across the compaction, reconciling the job pending meanwhile
    const idling = loop.idle();
    release.resolve();
    await idling;
    expect(reconciled).toEqual(["meanwhile"]);
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
        { controllers: [controller], cursors: [] },
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
        { controllers: [controller], cursors: [] },
        { report: ignore },
    );
    running(loop);
    await loop.idle();

    // count each note commit's transaction and its insert alone, past the log's first compaction
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "first", scope: "space" }).execute(),
    );
    await loop.idle();
    const before = database.driver.state.statements;
    for (let index = 0; index < NOTE_COMMITS; index++) {
        await database.transaction((transaction) =>
            transaction
                .insert(note)
                .values({ id: `note-${index}`, scope: "space" })
                .execute(),
        );
        await yieldToEventLoop();
    }
    expect(database.driver.state.statements - before).toBe(2 * NOTE_COMMITS);
});
