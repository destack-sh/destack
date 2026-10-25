import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Change, defineTable, eq, integer, lease, text } from "../index.ts";
import { reconciliation, TestDatabase } from "../test/index.ts";
import { testSimulation } from "@destack/test/world";
import { canonicalize, nth, found } from "@destack/schema";
import { ControlLoop, type Controller, type FollowController, Reconciliation } from "./control.ts";
import { until, yieldToEventLoop } from "@destack/environment";
import { SimulatedTelemetry, Simulation, SimulatedClock } from "@destack/environment/simulation";
import { Failure } from "@destack/error";
import { TestTelemetry } from "@destack/telemetry/test";

/** The commits of rows a follower's list never reads, about 30 ms of SQLite commits. */
const STRESS_COMMITS = 300;

/** How many notes a test commits one at a time. */
const NOTE_COMMITS = 20;

/** The jobs a controller reconciles. */
const job = defineTable(
    "controller_job",
    {
        /** The job's name. */
        id: text("id").primaryKey(),
        /** The space with the job. */
        scope: text("scope").notNull(),
        /** The desired number of runs. */
        runs: integer("runs").notNull(),
    },
    { log: {} },
);

/** Logged notes no controller watches. */
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

/** Unlogged claims one controller lists. */
const claim = defineTable("controller_claim", {
    /** The claim's name. */
    id: text("id").primaryKey(),
});

/** Unlogged tickets another controller lists. */
const ticket = defineTable("controller_ticket", {
    /** The ticket's name. */
    id: text("id").primaryKey(),
});

/** Read a reported failure's message, refusing a failure that is not an error. */
function message(error: unknown): string {
    if (!(error instanceof Error)) {
        throw new TypeError("a reconciliation failed with a value that is not an error");
    }

    return error.message;
}

/** Run a world until a read equals the expected value, then assert it whole. */
async function settle<Value>(simulation: Simulation, read: () => Value, expected: Value) {
    await simulation.run({ until: () => canonicalize(read()) === canonicalize(expected) });
    expect(read()).toEqual(expected);
}

/** Ignore a failed reconciliation. */
function ignore(): void {}

test("reconcile listed keys, the keys of changes, keys due again, and refused keys on their next change", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const recorded = new SimulatedTelemetry(world);
    using telemetry = new TestTelemetry(recorded);
    const storage = await TestDatabase.create([job], {
        environment: { ...world, telemetry: recorded },
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "listed", scope: "space", runs: 1 }).execute(),
    );

    // fail "flaky" once and look at "timed" again once
    const reconciled: string[] = [];
    const reported: string[] = [];
    let isFlaky = true;
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: (change) => (Change.of(change, job) ? [Change.image(change).id] : []),
        list: async () => ["listed"],
        reconcile: async (key) => {
            reconciled.push(key);
            if (key === "flaky" && isFlaky) {
                isFlaky = false;
                throw Failure.error(Failure.http("CONFLICT", "unavailable"));
            }

            return key === "timed" && reconciled.filter((entry) => entry === "timed").length === 1
                ? 10
                : undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: world.clock,
        report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
        retry: { initialInterval: 5 },
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // reconcile listed and written keys
    await settle(simulation, () => reconciled.includes("listed"), true);
    await database.transaction((transaction) =>
        transaction
            .insert(job)
            .values([
                { id: "timed", scope: "space", runs: 1 },
                { id: "flaky", scope: "space", runs: 1 },
            ])
            .execute(),
    );
    await settle(simulation, () => reconciled.toSorted(), ["flaky", "listed", "timed", "timed"]);

    // reconcile the refused key again on its next change
    await database.transaction((transaction) =>
        transaction.update(job).set({ runs: 2 }).where(eq(job.id, "flaky")).execute(),
    );
    await settle(simulation, () => reconciled.toSorted(), [
        "flaky",
        "flaky",
        "listed",
        "timed",
        "timed",
    ]);

    // report the failure to the creator and record it in db's scope
    expect([reported, telemetry.take()]).toEqual([
        ["flaky: unavailable"],
        [{ scope: "@destack/db", name: "Failure", message: "unavailable" }],
    ]);
});

test("report a key failing transiently only once its retries last past the window, and any other failure at once", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // leave "peer" unavailable until told otherwise, fail "broken" once, and misroute "misrouted" once
    const clock = new SimulatedClock(Date.UTC(2026, 9, 1));
    const started = clock.now();
    const reported: [string, number][] = [];
    let isDown = true;
    let isBroken = true;
    let isMisrouted = true;
    const controller: Controller = {
        name: "job",
        watches: [],
        keys: () => [],
        list: async () => ["peer", "broken", "misrouted"],
        reconcile: async (key) => {
            if (key === "peer" && isDown) {
                throw Failure.error(Failure.http("SERVICE_UNAVAILABLE", "service unavailable"));
            } else if (key === "broken" && isBroken) {
                isBroken = false;
                throw Failure.error(Failure.http("CONFLICT", "broken"));
            } else if (key === "misrouted" && isMisrouted) {
                isMisrouted = false;
                throw Failure.error({
                    code: "MISDIRECTED_REQUEST",
                    reason: "PARTITION_MISDIRECTED",
                    domain: "@example/space",
                    message: "space lives in another partition",
                });
            }

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(storage.database, [controller], {
        clock,
        report: (_controller, key) => reported.push([key, clock.now() - started]),
        retry: { initialInterval: 5, maximumInterval: 5 },
        transientFor: 40,
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // report "broken" at once and retry the others, then report "peer" once its retries fail past the window
    await loop.idle();
    const atOnce = [...reported];
    for (let retried = 0; retried < 8; retried += 1) {
        clock.advance(5);
        await loop.idle();
    }
    isDown = false;
    clock.advance(5);
    await loop.idle();
    expect({ atOnce, reported }).toEqual({
        atOnce: [["broken", 0]],
        reported: [
            ["broken", 0],
            ["peer", 40],
        ],
    });
});

test("leave a refused key quiet until its next change, reporting it once, and retry an unavailable key on backoff", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction
            .insert(job)
            .values([
                { id: "denied", scope: "space", runs: 1 },
                { id: "down", scope: "space", runs: 1 },
            ])
            .execute(),
    );

    // refuse "denied" and leave "down" unavailable
    const clock = new SimulatedClock(Date.UTC(2026, 9, 1));
    const attempts: string[] = [];
    const reported: string[] = [];
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: (change) => (Change.of(change, job) ? [Change.image(change).id] : []),
        list: async () => ["denied", "down"],
        reconcile: async (key) => {
            attempts.push(key);
            if (key === "denied") {
                throw Failure.error({
                    code: "FORBIDDEN",
                    reason: "PERMISSION_DENIED",
                    domain: "@example/access",
                    message: "permission denied: bind",
                });
            }
            throw Failure.error(Failure.http("SERVICE_UNAVAILABLE", "service unavailable"));
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock,
        report: (_controller, key) => reported.push(key),
        retry: { initialInterval: 60_000, maximumInterval: 60_000 },
        transientFor: 60 * 60_000,
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // wait out three simulated hours a minute at a time
    await loop.idle();
    for (let minute = 0; minute < 180; minute += 1) {
        clock.advance(60_000);
        await loop.idle();
    }
    const waited = Map.groupBy(attempts, (key) => key);

    // change "denied", which runs it again without a second report
    await database.transaction((transaction) =>
        transaction.update(job).set({ runs: 2 }).where(eq(job.id, "denied")).execute(),
    );
    await expect.poll(() => attempts.filter((key) => key === "denied").length).toBe(2);
    await loop.idle();

    // try "denied" once and "down" every minute, reporting each once
    expect({
        denied: waited.get("denied")?.length,
        down: waited.get("down")?.length,
        reported,
    }).toEqual({ denied: 1, down: 181, reported: ["denied", "down"] });
});

test("fail the loop, its run and its idle waiters, once a controller throws a failure it declares no outcome for", async () => {
    // reconcile one key in a world, its controller throwing a TypeError
    await using simulation = testSimulation();
    const environment = simulation.environment("controller");
    const storage = await TestDatabase.create([job], { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const defect = new TypeError("undefined is not an object");
    const controller: Controller = {
        name: "job",
        watches: [],
        keys: () => [],
        list: async () => ["broken"],
        reconcile: async () => {
            throw defect;
        },
    };
    const reported: string[] = [];
    const loop = new ControlLoop(storage.database, [controller], {
        clock: environment.clock,
        report: (_controller, key) => reported.push(key),
    });
    const running = loop.run(new AbortController().signal);

    // reject the driven idle wait and the run with the failure, naming the controller and the key
    const idled = await refusal(simulation.drive(loop.idle()));
    const ran = await running.catch((error: unknown) => error);

    expect({
        idled,
        ran: [ran instanceof Error ? ran.message : "", ran instanceof Error ? ran.cause : ""],
        reported,
    }).toEqual({
        idled: ["INTERNAL_SERVER_ERROR", "CONTROLLER_FAILED", "job failed to reconcile broken"],
        ran: ["job failed to reconcile broken", defect],
        reported: [],
    });
});

test("end a run whose loop fails, only after its running reconciliations ended", async () => {
    // reconcile a key held at a gate, its controller watching the log
    await using simulation = testSimulation();
    const environment = simulation.environment("controller");
    const storage = await TestDatabase.create([job], { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const events: string[] = [];
    const started = Promise.withResolvers<void>();
    const gate = Promise.withResolvers<void>();
    const held: Controller = {
        name: "held",
        watches: [job],
        keys: () => [],
        list: async () => ["held"],
        reconcile: async () => {
            events.push("held started");
            started.resolve();
            await gate.promise;
            events.push("held ended");

            return undefined;
        },
    };
    const loop = new ControlLoop(storage.database, [held], {
        clock: environment.clock,
        report: () => {},
    });
    const running = loop.run(new AbortController().signal).catch(() => events.push("run ended"));

    // close the database under the watching loop while the key reconciles, then release it
    await simulation.drive(started.promise);
    await storage.database.close();
    await simulation.run({ to: simulation.clock.now() + 1000 });
    gate.resolve();
    await simulation.drive(running);

    expect(events).toEqual(["held started", "held ended", "run ended"]);
});

test("retry a stale or moving key until it succeeds, reporting nothing", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // refuse "stale" once as a stale replica and once as a scope moving to another machine
    const attempts: string[] = [];
    const reported: string[] = [];
    const controller: Controller = {
        name: "job",
        watches: [],
        keys: () => [],
        list: async () => ["stale"],
        reconcile: async (key) => {
            attempts.push(key);
            if (attempts.length === 1) {
                throw Failure.error({
                    code: "SERVICE_UNAVAILABLE",
                    reason: "STALE",
                    domain: "@example/access",
                    message: "access was last confirmed by its home 600000 ms ago",
                });
            } else if (attempts.length === 2) {
                throw Failure.error({
                    code: "MISDIRECTED_REQUEST",
                    reason: "SCOPE_MOVED",
                    domain: "@example/object",
                    message: "space moved to machine-b",
                    data: { scope: "space", machine: "machine-b" },
                });
            }

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(storage.database, [controller], {
        clock: world.clock,
        report: (_controller, key) => reported.push(key),
        retry: { initialInterval: 5, maximumInterval: 5 },
        transientFor: 60_000,
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // succeed on the third attempt without a report
    await settle(simulation, () => attempts.length, 3);
    expect({ attempts, reported }).toEqual({ attempts: ["stale", "stale", "stale"], reported: [] });
});

test("report a key function that fails on a change and reconcile every listed key in its place, following later changes", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // key each changed job
    const reconciled: string[] = [];
    const reported: string[] = [];
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: (change) => {
            const id = Change.of(change, job) ? Change.image(change).id : "";
            if (id === "broken") {
                throw new Error("unreadable change");
            }

            return [id];
        },
        list: async () => (await database.select().from(job)).map((row) => row.id),
        reconcile: async (key) => {
            reconciled.push(key);

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: world.clock,
        report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await simulation.drive(loop.idle());

    // report the failing change and reconcile the listed keys
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "broken", scope: "space", runs: 1 }).execute(),
    );
    await settle(simulation, () => reconciled, ["broken"]);
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "later", scope: "space", runs: 1 }).execute(),
    );
    await settle(simulation, () => reconciled.toSorted(), ["broken", "later"]);
    expect(reported).toEqual(["controller_job: unreadable change"]);
});

test("list a controller of an unlogged table again after a commit to its table alone", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([claim, ticket], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // count each controller's listings
    const listings = { claims: 0, tickets: 0 };
    const claims: Controller = {
        name: "claims",
        watches: [claim],
        list: async () => {
            listings.claims += 1;

            return [];
        },
        reconcile: async () => undefined,
    };
    const tickets: Controller = {
        name: "tickets",
        watches: [ticket],
        list: async () => {
            listings.tickets += 1;

            return [];
        },
        reconcile: async () => undefined,
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [claims, tickets], {
        clock: world.clock,
        report: ignore,
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // list each controller at the start and once more before any commit
    await settle(simulation, () => ({ ...listings }), { claims: 2, tickets: 2 });

    // list the tickets again after a ticket commit, in the same pass leaving the claims alone
    await database.insert(ticket).values({ id: "first" });
    await settle(simulation, () => ({ ...listings }), { claims: 2, tickets: 3 });

    // list the claims again after a claim commit, leaving the tickets alone
    await database.insert(claim).values({ id: "first" });
    await settle(simulation, () => ({ ...listings }), { claims: 3, tickets: 3 });
});

test("list again after compaction only the controllers following the compacted tables, leaving a periodic one to its delay, and settle idle once relisted", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // stop the follower at its first change until the changes after it are compacted
    const listed: string[] = [];
    const swept: string[] = [];
    const stopped = Promise.withResolvers<void>();
    const release = Promise.withResolvers<void>();
    const jobs: Controller = {
        name: "jobs",
        watches: [job],
        keys: async () => {
            stopped.resolve();
            await release.promise;

            return [];
        },
        list: async () => {
            listed.push("jobs");

            return [];
        },
        reconcile: async () => undefined,
    };
    const sweep: Controller = {
        name: "sweep",
        list: async () => {
            listed.push("sweep");

            return ["stores"];
        },
        reconcile: async (key) => {
            swept.push(key);

            return 60_000;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [jobs, sweep], { clock: world.clock, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await settle(simulation, () => swept, ["stores"]);

    // commit a change the follower stops at, then another, and compact both
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "first", scope: "space", runs: 1 }).execute(),
    );
    await stopped.promise;
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "second", scope: "space", runs: 1 }).execute(),
    );
    const now = world.clock.now();
    await database.log.compact(now + 1, now);

    // wait idle across the compaction, which settles once the follower lists again
    const idling = loop.idle();
    await yieldToEventLoop();
    release.resolve();
    await idling;

    // list the jobs again, and leave the sweep to the delay it returned
    expect({ listed, swept }).toEqual({ listed: ["jobs", "sweep", "jobs"], swept: ["stores"] });
});

test("reconcile keys up to a controller's concurrency, never one key twice at once", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // block each reconciliation until released
    const releases = new Map<string, () => void>();
    let running = 0;
    let most = 0;
    const started: string[] = [];
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: () => [],
        list: async () => ["a", "b", "c"],
        concurrency: 2,
        reconcile: async (key) => {
            started.push(key);
            running += 1;
            most = Math.max(most, running);
            await new Promise<void>((release) => {
                releases.set(key, release);
            });
            running -= 1;

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(storage.database, [controller], {
        clock: world.clock,
        report: () => {},
    });
    const looping = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        releases.forEach((release) => release());
        await looping;
    });

    // enqueue the first key again while it runs and finish the second
    await settle(simulation, () => started.length, 2);
    const first = nth(started, 0);
    const second = nth(started, 1);
    const third = nth(
        ["a", "b", "c"].filter((key) => !started.includes(key)),
        0,
    );
    loop.enqueue(controller, first);
    found(releases, second)();
    await settle(simulation, () => started, [first, second, third]);

    // reconcile the first again after it finishes
    found(releases, first)();
    await settle(simulation, () => started, [first, second, third, first]);
    expect(most).toBe(2);
});

test("lease each key to one instance of several sharing a database, handing it over once its holder stops", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([job, lease], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await storage.database.transaction((transaction) =>
        transaction.insert(job).values({ id: "shared", scope: "space", runs: 1 }).execute(),
    );

    // run the loop on two instances
    const reconciled: string[] = [];
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: () => [],
        list: async () => ["shared"],
        reconcile: async (_key, { epoch }) => {
            reconciled.push(`${epoch}`);

            return undefined;
        },
    };
    const clock = new SimulatedClock(Date.UTC(2026, 9, 24));
    const first = new AbortController();
    const second = new AbortController();
    const running = [
        new ControlLoop(storage.database, [controller], {
            clock,
            report: ignore,
            lease: { holder: "first", duration: 200 },
        }).run(first.signal),
        new ControlLoop(storage.database, [controller], {
            clock,
            report: ignore,
            lease: { holder: "second", duration: 200 },
        }).run(second.signal),
    ];
    onTestFinished(async () => {
        first.abort();
        second.abort();
        await Promise.all(running);
    });

    // reconcile the key once on the lease holder
    await expect.poll(() => reconciled.length).toBe(1);

    // stop the holder, then let the other instance take the released key afresh
    const leases = await storage.database.select({ holder: lease.holder }).from(lease);
    const isFirst = nth(leases, 0).holder === "first";
    (isFirst ? first : second).abort();
    await nth(running, isFirst ? 0 : 1);
    clock.advance(200);
    await expect.poll(() => reconciled, { timeout: 2000 }).toEqual(["1", "1"]);
});

test("follow each listed key until its list drops it or the loop stops", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction
            .insert(job)
            .values([
                { id: "stayed", scope: "space", runs: 1 },
                { id: "dropped", scope: "space", runs: 1 },
            ])
            .execute(),
    );

    // follow each job until its signal aborts
    const started: string[] = [];
    const stopped: string[] = [];
    const reported: string[] = [];
    const controller: FollowController = {
        name: "follow",
        mode: "follow",
        watches: [job],
        list: async (view) => (await view.read(job).findMany()).map((row) => row.id),
        concurrency: 8,
        reconcile: async (key, { signal, synced }) => {
            started.push(key);
            synced();
            await new Promise<void>((resolve) => {
                signal.addEventListener("abort", () => resolve(), { once: true });
            });
            stopped.push(key);
            throw signal.reason;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: world.clock,
        report: (_controller, key) => reported.push(key),
    });
    const running = loop.run(stopping.signal);

    // stop the key the list drops and start the one it adds
    await settle(simulation, () => started.toSorted(), ["dropped", "stayed"]);
    await database.transaction((transaction) =>
        transaction.delete(job).where(eq(job.id, "dropped")).execute(),
    );
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "added", scope: "space", runs: 1 }).execute(),
    );
    await settle(simulation, () => stopped, ["dropped"]);
    await settle(simulation, () => started.toSorted(), ["added", "dropped", "stayed"]);

    // settle idle while the follows run
    await simulation.drive(loop.idle());
    expect(stopped).toEqual(["dropped"]);

    // stop the rest with the loop
    stopping.abort();
    await running;
    expect(stopped.toSorted()).toEqual(["added", "dropped", "stayed"]);
    expect(reported).toEqual([]);
});

test("settle idle once each follow synced while reporting a follow failing before it syncs", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // sync one follow once its gate opens, and fail the other before it syncs
    const gate = Promise.withResolvers<void>();
    const started: string[] = [];
    const controller: FollowController = {
        name: "follow",
        mode: "follow",
        concurrency: 2,
        list: async () => ["syncing", "failing"],
        reconcile: async (key, { signal, synced }) => {
            started.push(key);
            if (key === "failing") {
                throw Failure.error(Failure.http("CONFLICT", "the source refused the follow"));
            }
            await gate.promise;
            synced();
            await new Promise<void>((resolve) => {
                signal.addEventListener("abort", () => resolve(), { once: true });
            });
            throw signal.reason;
        },
    };
    const stopping = new AbortController();
    const reported: string[] = [];
    const loop = new ControlLoop(storage.database, [controller], {
        clock: new SimulatedClock(Date.UTC(2026, 9, 1)),
        report: (failed, key, error) => {
            reported.push(
                `${failed.name} ${key}: ${error instanceof Error ? error.message : "not an error"}`,
            );
        },
        retry: { initialInterval: 1000, maximumInterval: 1000 },
    });
    const running = loop.run(stopping.signal);

    // stay unsettled while the follow waits at its gate and the failed one waits out its retry, then settle once it syncs
    await expect.poll(() => started.includes("syncing") && reported.length === 1).toBe(true);
    const depthAtGate = loop.depth;
    gate.resolve();
    await loop.idle();
    expect([depthAtGate, loop.depth, [...reported]]).toEqual([
        1,
        0,
        ["follow failing: the source refused the follow"],
    ]);

    // stop the follow with the loop
    stopping.abort();
    await running;
});

test("list a following controller again only for a commit of a row its list read, past many it never read", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // follow the space's jobs, counting the listings
    let listings = 0;
    const controller: FollowController = {
        name: "follow",
        mode: "follow",
        watches: [job],
        list: async (view) => {
            listings += 1;

            return (await view.read(job).findMany({ where: { scope: "space" } })).map(
                (row) => row.id,
            );
        },
        reconcile: async (_key, { signal }) => {
            await until(signal);
            throw signal.reason;
        },
    };
    // see each change the loop reads, through a reconciler listed after the follower it waits on
    const seen: unknown[] = [];
    const watcher: Controller = {
        name: "watcher",
        watches: [job],
        keys: (change) => {
            seen.push(Change.after(change)?.["id"]);

            return [];
        },
        list: async () => [],
        reconcile: async () => undefined,
    };
    const stopping = new AbortController();
    const reported: unknown[] = [];
    const loop = new ControlLoop(database, [controller, watcher], {
        clock: world.clock,
        report: (_controller, _key, error) => reported.push(error),
    });
    const running = loop.run(stopping.signal);
    await settle(simulation, () => listings, 1);

    // commit jobs of another scope one at a time, listing nothing again once the loop read them all
    for (let index = 0; index < STRESS_COMMITS; index++) {
        await database.transaction(
            async (transaction) =>
                await transaction
                    .insert(job)
                    .values({ id: `other-${index}`, scope: "other", runs: 1 }),
        );
    }
    await settle(simulation, () => seen.includes(`other-${STRESS_COMMITS - 1}`), true);
    const unrelated = listings;

    // add a job of the space
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "space-job", scope: "space", runs: 1 }).execute(),
    );
    await settle(simulation, () => listings, 2);
    stopping.abort();
    await running;

    // list again for the space's job alone, reporting nothing
    expect({ unrelated, listings, reported }).toEqual({ unrelated: 1, listings: 2, reported: [] });
});

test("fail the loop once a follow ends unasked", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "brief", scope: "space", runs: 1 }).execute(),
    );

    // end each follow at once
    const reported: string[] = [];
    const controller: FollowController = {
        name: "brief",
        mode: "follow",
        watches: [job],
        list: async (view) => (await view.read(job).findMany()).map((row) => row.id),
        reconcile: async () => undefined,
    };
    const loop = new ControlLoop(database, [controller], {
        clock: world.clock,
        report: (_controller, key) => reported.push(key),
    });

    // reject the run with the ended follow as its cause, reporting nothing
    const ran = await loop.run(new AbortController().signal).catch((error: unknown) => error);
    const cause = ran instanceof Error ? ran.cause : undefined;
    expect({
        ran: ran instanceof Error ? ran.message : "",
        cause: cause instanceof Error ? cause.message : "",
        reported,
    }).toEqual({
        ran: "brief failed to reconcile brief",
        cause: "brief stopped following brief without an abort",
        reported: [],
    });
});

test("retry a failing follow at once when its revision changes, else only after its backoff", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "job", scope: "space", runs: 1 }).execute(),
    );

    // fail each follow at once, the job's runs its revision
    const revisions = new Map<string, string>();
    const started: string[] = [];
    let listings = 0;
    const controller: FollowController = {
        name: "failing",
        mode: "follow",
        watches: [job],
        list: async (view) => {
            const rows = await view.read(job).findMany();
            for (const row of rows) {
                revisions.set(row.id, String(row.runs));
            }
            listings += 1;

            return rows.map((row) => row.id);
        },
        revision: (key) => revisions.get(key),
        reconcile: async (key) => {
            started.push(`${key}@${found(revisions, key)}`);
            throw Failure.error(Failure.http("CONFLICT", "refused"));
        },
    };

    // run the loop at a frozen clock, so no backoff ever ends
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: new SimulatedClock(Date.UTC(2026, 9, 24)),
        report: ignore,
    });
    const running = loop.run(stopping.signal);
    await expect.poll(() => started).toEqual(["job@1"]);

    // change the job without its revision, which lists it again without a retry
    const before = listings;
    await database.transaction((transaction) =>
        transaction.update(job).set({ scope: "elsewhere" }).where(eq(job.id, "job")).execute(),
    );
    await expect.poll(() => listings).toBe(before + 1);
    await yieldToEventLoop();
    const unrevised = [...started];

    // change its revision, which retries it at once
    await database.transaction((transaction) =>
        transaction.update(job).set({ runs: 2 }).where(eq(job.id, "job")).execute(),
    );
    await expect.poll(() => started).toEqual(["job@1", "job@2"]);
    expect(unrevised).toEqual(["job@1"]);
    stopping.abort();
    await running;
});

test("start a failing follow at once once its list drops it and lists it again, forgetting its failures", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "job", scope: "space", runs: 1 }).execute(),
    );

    // fail each follow at once, on a frozen clock so no backoff ends
    const started: string[] = [];
    let listings = 0;
    const controller: FollowController = {
        name: "failing",
        mode: "follow",
        watches: [job],
        list: async (view) => {
            listings += 1;

            return (await view.read(job).findMany()).map((row) => row.id);
        },
        reconcile: async (key) => {
            started.push(key);
            throw Failure.error(Failure.http("CONFLICT", "refused"));
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: new SimulatedClock(Date.UTC(2026, 9, 24)),
        report: ignore,
    });
    const running = loop.run(stopping.signal);
    await expect.poll(() => started).toEqual(["job"]);

    // drop the job from the list, then list it again
    const before = listings;
    await database.transaction((transaction) =>
        transaction.delete(job).where(eq(job.id, "job")).execute(),
    );
    await expect.poll(() => listings).toBe(before + 1);
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "job", scope: "space", runs: 1 }).execute(),
    );
    await expect.poll(() => started).toEqual(["job", "job"]);
    stopping.abort();
    await running;
});

test("refuse a follower whose list never reads its watched tables through its view", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // list a fixed key while watching jobs
    const controller: FollowController = {
        name: "blind",
        mode: "follow",
        watches: [job],
        list: async () => ["blind"],
        reconcile: async () => undefined,
    };
    const loop = new ControlLoop(storage.database, [controller], {
        clock: world.clock,
        report: () => {},
    });

    // fail the loop instead of never listing again
    await expect(loop.run(new AbortController().signal)).rejects.toEqual(
        new TypeError("blind watches tables, but its list reads no table"),
    );
});

test("follow a key again once its list names it after it failed and was dropped", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "flaky", scope: "space", runs: 1 }).execute(),
    );

    // fail the first follow and follow until stopped
    const started: string[] = [];
    const controller: FollowController = {
        name: "flaky",
        mode: "follow",
        watches: [job],
        concurrency: 8,
        list: async (view) => (await view.read(job).findMany()).map((row) => row.id),
        reconcile: async (key, { signal }) => {
            started.push(key);
            if (started.length === 1) {
                throw Failure.error(Failure.http("CONFLICT", "the first follow fails"));
            }
            await new Promise<void>((resolve) => {
                signal.addEventListener("abort", () => resolve(), { once: true });
            });
            throw signal.reason;
        },
    };
    const stopping = new AbortController();
    const running = new ControlLoop(database, [controller], {
        clock: world.clock,
        report: () => {},
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // leave the refused follow failed, and follow the key again once listed
    await settle(simulation, () => started, ["flaky"]);
    await database.transaction((transaction) => transaction.delete(job).execute());
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "other", scope: "space", runs: 1 }).execute(),
    );
    await settle(simulation, () => started, ["flaky", "other"]);
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "flaky", scope: "space", runs: 1 }).execute(),
    );
    await settle(simulation, () => started, ["flaky", "other", "flaky"]);
});

test("set the host's alarm at the earliest due or skipped key, clear it once nothing is due, and settle idle waiters, also after the loop stopped", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // look at "later" again after a moment and at "soon" never again
    const alarms: (number | null)[] = [];
    const reconciled: string[] = [];
    const controller: Controller = {
        name: "job",
        list: async () => ["later", "soon"],
        reconcile: async (key) => {
            reconciled.push(key);

            return key === "later" && reconciled.filter((entry) => entry === "later").length === 1
                ? 50
                : undefined;
        },
    };
    const clock = new SimulatedClock(Date.UTC(2026, 9, 24));
    const started = clock.now();
    const loop = new ControlLoop(storage.database, [controller], {
        clock,
        report: (_controller, _key, error) => {
            throw error;
        },
        alarm: {
            setAlarm: async (at) => {
                alarms.push(at);
            },
            deleteAlarm: async () => {
                alarms.push(null);
            },
        },
    });
    const stopping = new AbortController();
    const running = loop.run(stopping.signal);

    // settle once both keys ran, "later" not due again while the clock stands
    await loop.idle();
    const settled = [reconciled.toSorted(), alarms.length];

    // clear the alarm once the clock passed "later" again and nothing is due
    clock.advance(50);
    await expect.poll(() => alarms.at(-1), { interval: 5 }).toBeNull();
    stopping.abort();
    await running;
    expect([settled, reconciled.length, alarms.length]).toEqual([[["later", "soon"], 2], 3, 3]);
    expect(alarms).toEqual([started, started + 50, null]);

    // settle a waiter at once after the loop stopped
    await expect(loop.idle()).resolves.toBeUndefined();
});

test("settle idle only after reconciling the keys of every watched change committed before the call", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const reconciled: string[] = [];
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: (change) => (Change.of(change, job) ? [Change.image(change).id] : []),
        list: async () => [],
        reconcile: async (key) => {
            reconciled.push(key);

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], { clock: world.clock, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await simulation.drive(loop.idle());

    // commit changes one after another, each followed by an idle wait seeing its key reconciled
    const seen: string[][] = [];
    for (const id of ["first", "second", "third"]) {
        await database.transaction((transaction) =>
            transaction.insert(job).values({ id, scope: "space", runs: 1 }).execute(),
        );
        await simulation.drive(loop.idle());
        seen.push([...reconciled]);
    }
    expect(seen).toEqual([["first"], ["first", "second"], ["first", "second", "third"]]);
});

test("keep the follow cursor before a change the follow is still keying, and move it to the head once the key ran", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // block the follow inside the keys of a change until released
    const reconciled: string[] = [];
    const keying = Promise.withResolvers<void>();
    const release = Promise.withResolvers<void>();
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: async (change) => {
            keying.resolve();
            await release.promise;

            return Change.of(change, job) ? [Change.image(change).id] : [];
        },
        list: async () => [],
        reconcile: async (key) => {
            reconciled.push(key);

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], { clock: world.clock, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await simulation.drive(loop.idle());

    // commit a change and read the cursor while the follow keys it
    const { sequence: before } = await database.log.position();
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "changed", scope: "space", runs: 1 }).execute(),
    );
    await keying.promise;
    const { sequence: head } = await database.log.position();
    const followedWhileKeying = loop.followed;

    // release the follow and settle once the key ran, the follow at the head
    release.resolve();
    await simulation.drive(loop.idle());
    expect({ followedWhileKeying, followed: loop.followed, depth: loop.depth, reconciled }).toEqual(
        {
            followedWhileKeying: before,
            followed: head,
            depth: 0,
            reconciled: ["changed"],
        },
    );
});

test("spend no statement on commits to tables no controller watches", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job, note], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: (change) => (Change.of(change, job) ? [Change.image(change).id] : []),
        list: async () => [],
        reconcile: async () => undefined,
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], { clock: world.clock, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await simulation.drive(loop.idle());

    // commit notes one at a time for the loop to see, and count every statement
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
    // count each commit's transaction and its insert alone
    expect(database.driver.state.statements - before).toBe(2 * NOTE_COMMITS);
});

test("settle idle once the log moved past a quiet watched table, also after compacting its last change", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job, note], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: (change) => (Change.of(change, job) ? [Change.image(change).id] : []),
        list: async () => [],
        reconcile: async () => undefined,
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], { clock: world.clock, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // change the watched table once, then only another one, compacting past the watched change
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "first", scope: "space", runs: 1 }).execute(),
    );
    await simulation.drive(loop.idle());
    for (const id of ["one", "two", "three"]) {
        await database.transaction((transaction) =>
            transaction.insert(note).values({ id, scope: "space" }).execute(),
        );
    }
    const now = world.clock.now();
    await database.log.compact(now + 1, now);

    // settle with the follow at the head
    await expect(loop.idle()).resolves.toBeUndefined();
});

test("settle idle only after the key of a watched change committed while the alarm was being set", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // look at "later" again in a minute, which sets the alarm
    const reconciled: string[] = [];
    const keyed = Promise.withResolvers<void>();
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: (change) => {
            keyed.resolve();

            return Change.of(change, job) ? [Change.image(change).id] : [];
        },
        list: async () => ["later"],
        reconcile: async (key) => {
            reconciled.push(key);

            return key === "later" ? 60_000 : undefined;
        },
    };

    // commit a watched change while the alarm for "later" is set, blocked until the follow keyed it
    let isChanged = false;
    const alarm = {
        setAlarm: async () => {
            if (!isChanged && reconciled.length === 1) {
                isChanged = true;
                await database.transaction(
                    async (transaction) =>
                        await transaction
                            .insert(job)
                            .values({ id: "changed", scope: "space", runs: 1 }),
                );
                await keyed.promise;
                await yieldToEventLoop();
            }
        },
        deleteAlarm: async () => {},
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: world.clock,
        report: ignore,
        alarm,
    });
    const idling = loop.idle();
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // settle once the changed key ran too
    await idling;
    expect(reconciled).toEqual(["later", "changed"]);
});

test("reconcile a key named due while the worker's alarm is set, before it sleeps", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // mark "late" due while the worker's alarm is set at "first"
    const reconciled: string[] = [];
    const { promise: ran, resolve: run } = Promise.withResolvers<void>();
    const controller: Controller = {
        name: "job",
        list: async () => ["first"],
        reconcile: async (key) => {
            reconciled.push(key);
            run();

            return undefined;
        },
    };
    const loop: ControlLoop = new ControlLoop(storage.database, [controller], {
        clock: world.clock,
        report: (_controller, _key, error) => {
            throw error;
        },
        alarm: {
            setAlarm: async () => {
                // enqueue it once
                await ran;
                await yieldToEventLoop();
                if (!reconciled.includes("late")) {
                    loop.enqueue(controller, "late");
                }
            },
            deleteAlarm: async () => {},
        },
    });
    const stopping = new AbortController();
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // reconcile "late" without another change waking the worker
    await settle(simulation, () => reconciled, ["first", "late"]);
});

test("settle an alarm at its deadline while a follow runs, leave the alarm due, and resume the follow from its recorded position after eviction", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const storage = await TestDatabase.create([job], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction.insert(job).values({ id: "copy", scope: "space", runs: 0 }).execute(),
    );

    // follow a growing source
    const source = ["first", "second"];
    const applied: string[] = [];
    let follows = 0;
    const controller: FollowController = {
        name: "copy",
        mode: "follow",
        list: async () => ["copy"],
        reconcile: async (key, { signal }) => {
            follows++;
            // apply the events after the recorded position, then follow until stopped
            const row = await database.select().from(job).where(eq(job.id, key)).get();
            applied.push(...source.slice(row?.runs ?? 0));
            await database.transaction(
                async (transaction) =>
                    await transaction
                        .update(job)
                        .set({ runs: source.length })
                        .where(eq(job.id, key)),
            );
            await new Promise<void>((resolve) => {
                signal.addEventListener("abort", () => resolve(), { once: true });
            });
            throw signal.reason;
        },
    };

    // run the follow in a loop on the object's alarm
    const alarms: number[] = [];
    const alarm = {
        setAlarm: async (at: number) => {
            alarms.push(at);
        },
        deleteAlarm: async () => {
            throw new Error("a running follow leaves the alarm due");
        },
    };
    const start = () => {
        const stopping = new AbortController();
        const loop = new ControlLoop(database, [controller], {
            clock: world.clock,
            report: ignore,
            alarm,
        });

        return { loop, stopping, running: loop.run(stopping.signal) };
    };
    const evicted = start();

    // settle the alarm at its deadline once the follow runs
    await settle(simulation, () => follows, 1);
    await simulation.drive(evicted.loop.idle(world.clock.now() + 20));
    const settled = world.clock.now();
    const isDue = alarms.length === 1 && nth(alarms, 0) <= settled;

    // evict the object
    evicted.stopping.abort();
    await evicted.running;

    // receive another event and resume in a new loop from the recorded position
    source.push("third");
    const resumed = start();
    await settle(simulation, () => applied.length, 3);
    resumed.stopping.abort();
    await resumed.running;
    expect({ isDue, applied, follows }).toEqual({
        isDue: true,
        applied: ["first", "second", "third"],
        follows: 2,
    });
});

test("run work until a change shows it ended elsewhere, and settle unchanged work as it finishes", async () => {
    // run a reconciliation whose key changes when told to
    const changes: (() => void)[] = [];
    const changing = {
        ...reconciliation(),
        changed: () =>
            new Promise<void>((resolve) => {
                changes.push(resolve);
            }),
    };
    const change = () => changes.splice(0).forEach((resolve) => resolve());

    // abort work once a change shows it was cancelled
    let isCancelled = false;
    const cancelled = Reconciliation.runUntil(
        changing,
        async () => (isCancelled ? new Error("cancelled") : undefined),
        (signal) =>
            new Promise<unknown>((resolve) => {
                signal.addEventListener("abort", () => resolve(signal.reason), { once: true });
            }),
    );
    change();
    await yieldToEventLoop();
    isCancelled = true;
    change();

    // settle work no change ended with its result
    const finished = Reconciliation.runUntil(
        changing,
        async () => undefined,
        async () => "built",
    );
    expect([await cancelled, await finished]).toEqual([new Error("cancelled"), "built"]);
});
