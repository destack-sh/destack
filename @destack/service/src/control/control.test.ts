import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, eq, integer, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ControlLoop, type Controller } from "./control.ts";
import { controllerLease } from "./lease.ts";

/** The jobs a controller reconciles. */
const job = defineTable(
    "controller_job",
    {
        /** The job's name. */
        id: text("id").primaryKey(),
        /** The space holding the job. */
        scope: text("scope").notNull(),
        /** The desired number of runs. */
        runs: integer("runs").notNull(),
    },
    { log: {} },
);

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
        keys: (change) => [String((change.after ?? change.before)!.id)],
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
        report: (_controller, key, error) => reported.push(`${key}: ${(error as Error).message}`),
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
        .poll(() => [...reconciled].sort(), { timeout: 5000 })
        .toEqual(["flaky", "flaky", "listed", "timed", "timed"]);
    expect(reported).toEqual(["flaky: unavailable"]);
});

test("reconcile keys up to a controller's concurrency, never one key twice at once", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());

    // hold each reconciliation until released
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
            await new Promise<void>((release) => releases.set(key, release));
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
    const [first, second] = started as [string, string];
    const third = ["a", "b", "c"].find((key) => !started.includes(key))!;
    loop.enqueue(controller, first);
    releases.get(second)!();
    await expect.poll(() => started).toEqual([first, second, third]);

    // reconcile the first again after it finishes
    releases.get(first)!();
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
            reconciled.push(`${reconciliation!.epoch}`);

            return undefined;
        },
    };
    const first = new AbortController();
    const second = new AbortController();
    const report = () => {};
    const running = [
        new ControlLoop(storage.database, [controller], {
            report,
            lease: { holder: "first", duration: 200 },
        }).run(first.signal),
        new ControlLoop(storage.database, [controller], {
            report,
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
    const [held] = await storage.database
        .select({ holder: controllerLease.holder })
        .from(controllerLease);
    (held!.holder === "first" ? first : second).abort();
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
            await new Promise<void>((resolve) =>
                signal.addEventListener("abort", () => resolve(), { once: true }),
            );
            stopped.push(key);
            throw signal.reason;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        report: (_controller, key) => reported.push(key),
    });
    const running = loop.run(stopping.signal);

    // start both, then stop the one the list drops and start the one it adds
    await expect.poll(() => [...started].sort()).toEqual(["dropped", "kept"]);
    await database.delete(job).where(eq(job.id, "dropped"));
    await database.insert(job).values({ id: "added", scope: "space", runs: 1 });
    await expect.poll(() => stopped).toEqual(["dropped"]);
    await expect.poll(() => [...started].sort()).toEqual(["added", "dropped", "kept"]);

    // stop the rest with the loop, reporting no failure
    stopping.abort();
    await running;
    expect([...stopped].sort()).toEqual(["added", "dropped", "kept"]);
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
        report: (_controller, key, error) => reported.push(`${key}: ${(error as Error).message}`),
        retry: { initialInterval: 50 },
    });
    const running = loop.run(stopping.signal);

    // follow twice, the second only after the backoff, reporting each end
    await expect.poll(() => reported.length).toBe(2);
    stopping.abort();
    await running;
    const failure = "brief: brief stopped following brief unasked";
    expect({ isBackedOff: started[1]! - started[0]! >= 50, reported }).toEqual({
        isBackedOff: true,
        reported: [failure, failure],
    });
});

test("follow a key again once its list names it after it failed and was dropped", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "flaky", scope: "space", runs: 1 });

    // fail the first follow, then follow until stopped, throwing the abort reason as streams do
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
            await new Promise<void>((resolve) =>
                signal.addEventListener("abort", () => resolve(), { once: true }),
            );
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

    // retry the failed follow, drop the key, then follow it again once listed
    await expect.poll(() => started).toEqual(["flaky", "flaky"]);
    await database.delete(job);
    await database.insert(job).values({ id: "other", scope: "space", runs: 1 });
    await expect.poll(() => started).toEqual(["flaky", "flaky", "other"]);
    await database.insert(job).values({ id: "flaky", scope: "space", runs: 1 });
    await expect.poll(() => started).toEqual(["flaky", "flaky", "other", "flaky"]);
});

test("keep the host's alarm at the earliest due or held key, clear it once nothing is due, and settle idle waiters, also after the loop stopped", async () => {
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

    // settle once both keys ran: the alarm held for "soon" while "later" ran, then waking the instance for "later"
    await loop.idle();
    const settled = [[...reconciled].sort(), alarms.length];

    // clear the alarm once "later" ran again and nothing is due
    await expect.poll(() => alarms.at(-1), { interval: 5 }).toBeNull();
    stopping.abort();
    await running;
    expect([settled, reconciled.length, alarms.length]).toEqual([[["later", "soon"], 2], 3, 3]);
    expect([alarms[0]! - started < 50, alarms[1]! - started >= 50]).toEqual([true, true]);

    // settle a waiter at once after the loop stopped
    await expect(loop.idle()).resolves.toBeUndefined();
});
