import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, eq, integer, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ControlLoop, type Controller, type Follower } from "./control.ts";
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
    const controller: Follower = {
        name: "follow",
        watches: [job],
        list: async () => (await database.select({ id: job.id }).from(job)).map((row) => row.id),
        concurrency: 8,
        follow: async (key, signal) => {
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
    const controller: Follower = {
        name: "brief",
        watches: [job],
        list: async () => ["brief"],
        follow: async () => {
            started.push(Date.now());
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
