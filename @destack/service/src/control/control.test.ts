import { expect, onTestFinished, test } from "@destack/test";
import { Change, defineTable, eq, integer, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { aligned, found } from "@destack/schema";
import { ControlLoop, type Controller, Reconciliation } from "./control.ts";
import { controllerLease } from "./lease.ts";

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

/** Read a reported failure's message, refusing a failure that is no error. */
function message(error: unknown): string {
    if (!(error instanceof Error)) {
        throw new TypeError("a reconciliation failed with a value that is no error");
    }

    return error.message;
}

/** Ignore a failed reconciliation. */
function ignore(): void {}

test("reconcile listed keys, keys changes name, keys due again, and failed keys after a backoff", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "listed", scope: "space", runs: 1 });

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
                throw new Error("unavailable");
            }

            return key === "timed" && reconciled.filter((entry) => entry === "timed").length === 1
                ? 10
                : undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
        retry: { initialInterval: 5 },
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // reconcile listed and written keys
    await expect.poll(() => reconciled.includes("listed")).toBe(true);
    await database.insert(job).values([
        { id: "timed", scope: "space", runs: 1 },
        { id: "flaky", scope: "space", runs: 1 },
    ]);
    await expect
        .poll(() => reconciled.toSorted(), { timeout: 5000 })
        .toEqual(["flaky", "flaky", "listed", "timed", "timed"]);
    expect(reported).toEqual(["flaky: unavailable"]);
});

test("reconcile keys up to a controller's concurrency, never one key twice at once", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
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
    const loop = new ControlLoop(storage.database, [controller], { report: () => {} });
    const looping = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        releases.forEach((release) => release());
        await looping;
    });

    // name the first key again while it runs and finish the second
    await expect.poll(() => started.length).toBe(2);
    const first = aligned(started, 0);
    const second = aligned(started, 1);
    const third = aligned(
        ["a", "b", "c"].filter((key) => !started.includes(key)),
        0,
    );
    loop.enqueue(controller, first);
    found(releases, second)();
    await expect.poll(() => started).toEqual([first, second, third]);

    // reconcile the first again after it finishes
    found(releases, first)();
    await expect.poll(() => started).toEqual([first, second, third, first]);
    expect(most).toBe(2);
});

test("lease each key to one instance of several sharing a database, handing it over once the lease lapses", async () => {
    const storage = await TestDatabase.create("sqlite", [job, controllerLease], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await storage.database.insert(job).values({ id: "shared", scope: "space", runs: 1 });

    // run the loop on two instances
    const reconciled: string[] = [];
    const controller: Controller = {
        name: "job",
        watches: [job],
        keys: () => [],
        list: async () => ["shared"],
        reconcile: async (_key, reconciliation) => {
            reconciled.push(`${reconciliation.epoch}`);

            return undefined;
        },
    };
    const first = new AbortController();
    const second = new AbortController();
    const running = [
        new ControlLoop(storage.database, [controller], {
            report: ignore,
            lease: { holder: "first", duration: 200 },
        }).run(first.signal),
        new ControlLoop(storage.database, [controller], {
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

    // stop the holder and let the other instance take the key
    const leases = await storage.database
        .select({ holder: controllerLease.holder })
        .from(controllerLease);
    (aligned(leases, 0).holder === "first" ? first : second).abort();
    await expect.poll(() => reconciled, { timeout: 2000 }).toEqual(["1", "1"]);
});

test("follow each listed key until its list drops it or the loop stops", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values([
        { id: "kept", scope: "space", runs: 1 },
        { id: "dropped", scope: "space", runs: 1 },
    ]);

    // follow each job until its signal aborts
    const started: string[] = [];
    const stopped: string[] = [];
    const reported: string[] = [];
    const controller: Controller = {
        name: "follow",
        mode: "follow",
        watches: [job],
        list: async () => (await database.select({ id: job.id }).from(job)).map((row) => row.id),
        concurrency: 8,
        reconcile: async (key, { signal }) => {
            started.push(key);
            await new Promise<void>((resolve) => {
                signal.addEventListener("abort", () => resolve(), { once: true });
            });
            stopped.push(key);
            throw signal.reason;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        report: (_controller, key) => reported.push(key),
    });
    const running = loop.run(stopping.signal);

    // start both, stop the one the list drops and start the one it adds
    await expect.poll(() => started.toSorted()).toEqual(["dropped", "kept"]);
    await database.delete(job).where(eq(job.id, "dropped"));
    await database.insert(job).values({ id: "added", scope: "space", runs: 1 });
    await expect.poll(() => stopped).toEqual(["dropped"]);
    await expect.poll(() => started.toSorted()).toEqual(["added", "dropped", "kept"]);

    // stop the rest with the loop, reporting no failure
    stopping.abort();
    await running;
    expect(stopped.toSorted()).toEqual(["added", "dropped", "kept"]);
    expect(reported).toEqual([]);
});

test("report a follow that ends unasked and retry it after a backoff", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "brief", scope: "space", runs: 1 });

    // end each follow at once, as a broken follower does
    const started: number[] = [];
    const reported: string[] = [];
    const controller: Controller = {
        name: "brief",
        mode: "follow",
        watches: [job],
        list: async () => ["brief"],
        reconcile: async () => {
            started.push(Date.now());

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
        retry: { initialInterval: 50 },
    });
    const running = loop.run(stopping.signal);

    // follow twice, the second only after the backoff, reporting each end
    await expect.poll(() => reported.length).toBe(2);
    stopping.abort();
    await running;
    const failure = "brief: brief stopped following brief unasked";
    expect({ isBackedOff: aligned(started, 1) - aligned(started, 0) >= 50, reported }).toEqual({
        isBackedOff: true,
        reported: [failure, failure],
    });
});

test("follow a key again once its list names it after it failed and was dropped", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "flaky", scope: "space", runs: 1 });

    // fail the first follow and follow until stopped, throwing the abort reason as streams do
    const started: string[] = [];
    const controller: Controller = {
        name: "flaky",
        mode: "follow",
        watches: [job],
        concurrency: 8,
        list: async () => (await database.select({ id: job.id }).from(job)).map((row) => row.id),
        reconcile: async (key, { signal }) => {
            started.push(key);
            if (started.length === 1) {
                throw new Error("the first follow fails");
            }
            await new Promise<void>((resolve) => {
                signal.addEventListener("abort", () => resolve(), { once: true });
            });
            throw signal.reason;
        },
    };
    const stopping = new AbortController();
    const running = new ControlLoop(database, [controller], {
        report: () => {},
        retry: { initialInterval: 10 },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // retry the failed follow, drop the key and follow it again once listed
    await expect.poll(() => started).toEqual(["flaky", "flaky"]);
    await database.delete(job);
    await database.insert(job).values({ id: "other", scope: "space", runs: 1 });
    await expect.poll(() => started).toEqual(["flaky", "flaky", "other"]);
    await database.insert(job).values({ id: "flaky", scope: "space", runs: 1 });
    await expect.poll(() => started).toEqual(["flaky", "flaky", "other", "flaky"]);
});

test("keep the host's alarm at the earliest due or skipped key, clear it once nothing is due, and settle idle waiters, also after the loop stopped", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());

    // look at "later" again after a moment, and at "soon" never again
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
    const started = Date.now();
    const loop = new ControlLoop(storage.database, [controller], {
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

    // settle once both keys ran: the alarm stayed at "soon" while "later" ran, waking the instance for "later" after
    await loop.idle();
    const settled = [reconciled.toSorted(), alarms.length];

    // clear the alarm once "later" ran again and nothing is due
    await expect.poll(() => alarms.at(-1), { interval: 5 }).toBeNull();
    stopping.abort();
    await running;
    expect([settled, reconciled.length, alarms.length]).toEqual([[["later", "soon"], 2], 3, 3]);
    const firstAlarm = aligned(alarms, 0);
    const secondAlarm = aligned(alarms, 1);
    expect([
        firstAlarm !== null && firstAlarm - started < 50,
        secondAlarm !== null && secondAlarm - started >= 50,
    ]).toEqual([true, true]);

    // settle a waiter at once after the loop stopped
    await expect(loop.idle()).resolves.toBeUndefined();
});

test("settle an alarm at its deadline while a follow runs, keep the alarm due, and resume the follow from its recorded position after eviction", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "copy", scope: "space", runs: 0 });

    // follow a growing source, recording the position reached in the job's row as a replica records its copy's
    const source = ["first", "second"];
    const applied: string[] = [];
    let follows = 0;
    const controller: Controller = {
        name: "copy",
        mode: "follow",
        list: async () => ["copy"],
        reconcile: async (key, { signal }) => {
            follows++;
            // apply the events after the recorded position and follow until stopped
            const row = await database.select().from(job).where(eq(job.id, key)).get();
            applied.push(...source.slice(row?.runs ?? 0));
            await database.update(job).set({ runs: source.length }).where(eq(job.id, key));
            await new Promise<void>((resolve) => {
                signal.addEventListener("abort", () => resolve(), { once: true });
            });
            throw signal.reason;
        },
    };

    // run the follow in a loop on the object's alarm, recording where the alarm is due
    const alarms: number[] = [];
    const alarm = {
        setAlarm: async (at: number) => {
            alarms.push(at);
        },
        deleteAlarm: async () => {
            throw new Error("a running follow keeps the alarm due");
        },
    };
    const start = () => {
        const stopping = new AbortController();
        const loop = new ControlLoop(database, [controller], { report: ignore, alarm });

        return { loop, stopping, running: loop.run(stopping.signal) };
    };
    const evicted = start();

    // settle the alarm at its deadline with the follow running, its wake-up due at once
    await evicted.loop.idle(Date.now() + 20);
    const settled = Date.now();
    const isDue = alarms.length === 1 && aligned(alarms, 0) <= settled;

    // evict the object
    evicted.stopping.abort();
    await evicted.running;

    // receive another event and resume in a new loop from the recorded position
    source.push("third");
    const resumed = start();
    await expect.poll(() => applied.length).toBe(3);
    resumed.stopping.abort();
    await resumed.running;
    expect({ isDue, applied, follows }).toEqual({
        isDue: true,
        applied: ["first", "second", "third"],
        follows: 2,
    });
});

test("run work until a change shows it ended elsewhere, and settle unchanged work as it finishes", async () => {
    // keep a reconciliation whose key changes when told to
    const changes: (() => void)[] = [];
    const reconciliation = {
        signal: new AbortController().signal,
        changed: () =>
            new Promise<void>((resolve) => {
                changes.push(resolve);
            }),
    };
    const change = () => changes.splice(0).forEach((resolve) => resolve());

    // abort work once a change shows it was cancelled, keeping the reason
    let isCancelled = false;
    const cancelled = Reconciliation.runUntil(
        reconciliation,
        async () => (isCancelled ? new Error("cancelled") : undefined),
        (signal) =>
            new Promise<unknown>((resolve) => {
                signal.addEventListener("abort", () => resolve(signal.reason), { once: true });
            }),
    );
    change();
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
    isCancelled = true;
    change();

    // settle work no change ended with its result
    const finished = Reconciliation.runUntil(
        reconciliation,
        async () => undefined,
        async () => "built",
    );
    expect([await cancelled, await finished]).toEqual([new Error("cancelled"), "built"]);
});
