import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, integer, text } from "@destack/db";
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

    // reconcile the first again once it finished
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
        reconcile: async (_key, lease) => {
            reconciled.push(`${lease!.epoch}`);

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
