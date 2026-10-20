import { expect, onTestFinished, test } from "@destack/test";
import { Change, defineTable, eq, integer, lease, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { nth, found } from "@destack/schema";
import { ControlLoop, type Controller, Reconciliation } from "./control.ts";
import { until, yieldToEventLoop } from "../timer/index.ts";
import { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";

/** The commits of rows a follower's list never reads, about 30 ms of SQLite commits. */
const STRESS_COMMITS = 300;

/** The event loop turns a test gives the loop to act on, ten times the few a settle takes. */
const QUIET_TURNS = 20;

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
        clock: Clock.system,
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

test("report a key whose peer answers unavailable only once it stays so past the window, and any other failure at once", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());

    // leave "peer" unavailable until told otherwise, and fail "broken" once
    const started = Date.now();
    const reported: [string, boolean][] = [];
    let isDown = true;
    let isBroken = true;
    const controller: Controller = {
        name: "job",
        watches: [],
        keys: () => [],
        list: async () => ["peer", "broken"],
        reconcile: async (key) => {
            if (key === "peer" && isDown) {
                throw Object.assign(new Error("service unavailable"), { status: 503 });
            } else if (key === "broken" && isBroken) {
                isBroken = false;
                throw new Error("broken");
            }

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(storage.database, [controller], {
        clock: Clock.system,
        report: (_controller, key) => reported.push([key, Date.now() - started >= 40]),
        retry: { initialInterval: 5, maximumInterval: 5 },
        unavailableFor: 40,
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // report "broken" at once, and "peer" only past the window
    await expect.poll(() => reported.some(([key]) => key === "peer")).toBe(true);
    isDown = false;
    expect(
        reported.filter(
            ([key], index) =>
                key === "broken" || index === reported.findIndex(([other]) => other === "peer"),
        ),
    ).toEqual([
        ["broken", false],
        ["peer", true],
    ]);
});

test("retry a reconciliation a domain failure refuses as unavailable, such as a stale replica, until it succeeds, reporting nothing", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());

    // refuse "stale" twice as access refuses a replica its home has not confirmed lately
    const attempts: string[] = [];
    const reported: string[] = [];
    const controller: Controller = {
        name: "job",
        watches: [],
        keys: () => [],
        list: async () => ["stale"],
        reconcile: async (key) => {
            attempts.push(key);
            if (attempts.length <= 2) {
                throw Object.assign(
                    new Error("access was last confirmed by its home 600000 ms ago"),
                    {
                        code: "STALE",
                        toCallError: () => ({
                            code: "SERVICE_UNAVAILABLE",
                            message: "access was last confirmed by its home 600000 ms ago",
                        }),
                    },
                );
            }

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(storage.database, [controller], {
        clock: Clock.system,
        report: (_controller, key) => reported.push(key),
        retry: { initialInterval: 5, maximumInterval: 5 },
        unavailableFor: 60_000,
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // succeed on the third attempt without a report
    await expect.poll(() => attempts.length).toBe(3);
    expect({ attempts, reported }).toEqual({ attempts: ["stale", "stale", "stale"], reported: [] });
});

test("report a key function that fails on a change and reconcile every listed key in its place, following later changes", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
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
        clock: Clock.system,
        report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await loop.idle();

    // report the failing change and reconcile the listed keys
    await database.insert(job).values({ id: "broken", scope: "space", runs: 1 });
    await expect.poll(() => reconciled).toEqual(["broken"]);
    await database.insert(job).values({ id: "later", scope: "space", runs: 1 });
    await expect.poll(() => reconciled.toSorted()).toEqual(["broken", "later"]);
    expect(reported).toEqual(["controller_job: unreadable change"]);
});

test("list a controller of an unlogged table again after a commit to its table alone", async () => {
    const storage = await TestDatabase.create("sqlite", [claim, ticket], { isMigrated: true });
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
        clock: Clock.system,
        report: ignore,
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // list each controller at the start and once more before any commit
    await expect.poll(() => ({ ...listings })).toEqual({ claims: 2, tickets: 2 });

    // list the tickets again after a ticket commit, in the same pass leaving the claims alone
    await database.insert(ticket).values({ id: "first" });
    await expect.poll(() => ({ ...listings })).toEqual({ claims: 2, tickets: 3 });

    // list the claims again after a claim commit, leaving the tickets alone
    await database.insert(claim).values({ id: "first" });
    await expect.poll(() => ({ ...listings })).toEqual({ claims: 3, tickets: 3 });
});

test("list again after compaction only the controllers following the compacted tables, leaving a periodic one to its delay, and settle idle once relisted", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
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
    const loop = new ControlLoop(database, [jobs, sweep], { clock: Clock.system, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await expect.poll(() => swept).toEqual(["stores"]);

    // commit a change the follower stops at, then another, and compact both
    await database.insert(job).values({ id: "first", scope: "space", runs: 1 });
    await stopped.promise;
    await database.insert(job).values({ id: "second", scope: "space", runs: 1 });
    const now = Date.now();
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
    const loop = new ControlLoop(storage.database, [controller], {
        clock: Clock.system,
        report: () => {},
    });
    const looping = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        releases.forEach((release) => release());
        await looping;
    });

    // enqueue the first key again while it runs and finish the second
    await expect.poll(() => started.length).toBe(2);
    const first = nth(started, 0);
    const second = nth(started, 1);
    const third = nth(
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
    const storage = await TestDatabase.create("sqlite", [job, lease], {
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
    const clock = new VirtualClock(Date.now());
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

    // stop the holder and let the other instance take the key
    const leases = await storage.database.select({ holder: lease.holder }).from(lease);
    (nth(leases, 0).holder === "first" ? first : second).abort();
    clock.advance(200);
    await expect.poll(() => reconciled, { timeout: 2000 }).toEqual(["1", "1"]);
});

test("follow each listed key until its list drops it or the loop stops", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values([
        { id: "stayed", scope: "space", runs: 1 },
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
        list: async (view) => (await view.read(job).findMany()).map((row) => row.id),
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
        clock: Clock.system,
        report: (_controller, key) => reported.push(key),
    });
    const running = loop.run(stopping.signal);

    // stop the key the list drops and start the one it adds
    await expect.poll(() => started.toSorted()).toEqual(["dropped", "stayed"]);
    await database.delete(job).where(eq(job.id, "dropped"));
    await database.insert(job).values({ id: "added", scope: "space", runs: 1 });
    await expect.poll(() => stopped).toEqual(["dropped"]);
    await expect.poll(() => started.toSorted()).toEqual(["added", "dropped", "stayed"]);

    // stop the rest with the loop
    stopping.abort();
    await running;
    expect(stopped.toSorted()).toEqual(["added", "dropped", "stayed"]);
    expect(reported).toEqual([]);
});

test("list a following controller again only for a commit of a row its list read, past many it never read", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // follow the space's jobs, counting the listings
    let listings = 0;
    const controller: Controller = {
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
        clock: Clock.system,
        report: (_controller, _key, error) => reported.push(error),
    });
    const running = loop.run(stopping.signal);
    await expect.poll(() => listings).toBe(1);

    // commit jobs of another scope one at a time, listing nothing again once the loop read them all
    for (let index = 0; index < STRESS_COMMITS; index++) {
        await database.insert(job).values({ id: `other-${index}`, scope: "other", runs: 1 });
    }
    await expect.poll(() => seen.includes(`other-${STRESS_COMMITS - 1}`)).toBe(true);
    const unrelated = listings;

    // add a job of the space
    await database.insert(job).values({ id: "space-job", scope: "space", runs: 1 });
    await expect.poll(() => listings).toBe(2);
    stopping.abort();
    await running;

    // list again for the space's job alone, reporting nothing
    expect({ unrelated, listings, reported }).toEqual({ unrelated: 1, listings: 2, reported: [] });
});

test("report a follow that ends unasked and retry it after a backoff", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "brief", scope: "space", runs: 1 });

    // end each follow at once
    const started: number[] = [];
    const reported: string[] = [];
    const controller: Controller = {
        name: "brief",
        mode: "follow",
        watches: [job],
        list: async (view) => (await view.read(job).findMany()).map((row) => row.id),
        reconcile: async () => {
            started.push(Date.now());

            return undefined;
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: Clock.system,
        report: (_controller, key, error) => reported.push(`${key}: ${message(error)}`),
        retry: { initialInterval: 50 },
    });
    const running = loop.run(stopping.signal);

    // follow twice with a backoff between
    await expect.poll(() => reported.length).toBe(2);
    stopping.abort();
    await running;
    const failure = "brief: brief stopped following brief without an abort";
    expect({ isBackedOff: nth(started, 1) - nth(started, 0) >= 50, reported }).toEqual({
        isBackedOff: true,
        reported: [failure, failure],
    });
});

test("retry a failing follow at once when its revision changes, else only after its backoff", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "job", scope: "space", runs: 1 });

    // fail each follow at once, the job's runs its revision
    const revisions = new Map<string, string>();
    const started: string[] = [];
    let listings = 0;
    const controller: Controller = {
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
            throw new Error("refused");
        },
    };

    // run the loop at a frozen clock, so no backoff ever ends
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: new VirtualClock(Date.now()),
        report: ignore,
    });
    const running = loop.run(stopping.signal);
    await expect.poll(() => started).toEqual(["job@1"]);

    // change the job without its revision, which lists it again without a retry
    const before = listings;
    await database.update(job).set({ scope: "elsewhere" }).where(eq(job.id, "job"));
    await expect.poll(() => listings).toBe(before + 1);
    await yieldToEventLoop();
    const unrevised = [...started];

    // change its revision, which retries it at once
    await database.update(job).set({ runs: 2 }).where(eq(job.id, "job"));
    await expect.poll(() => started).toEqual(["job@1", "job@2"]);
    expect(unrevised).toEqual(["job@1"]);
    stopping.abort();
    await running;
});

test("start a failing follow at once once its list drops it and lists it again, forgetting its failures", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "job", scope: "space", runs: 1 });

    // fail each follow at once, on a frozen clock so no backoff ends
    const started: string[] = [];
    let listings = 0;
    const controller: Controller = {
        name: "failing",
        mode: "follow",
        watches: [job],
        list: async (view) => {
            listings += 1;

            return (await view.read(job).findMany()).map((row) => row.id);
        },
        reconcile: async (key) => {
            started.push(key);
            throw new Error("refused");
        },
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: new VirtualClock(Date.now()),
        report: ignore,
    });
    const running = loop.run(stopping.signal);
    await expect.poll(() => started).toEqual(["job"]);

    // drop the job from the list, then list it again
    const before = listings;
    await database.delete(job).where(eq(job.id, "job"));
    await expect.poll(() => listings).toBe(before + 1);
    await database.insert(job).values({ id: "job", scope: "space", runs: 1 });
    await expect.poll(() => started).toEqual(["job", "job"]);
    stopping.abort();
    await running;
});

test("refuse a follower whose list never reads its watched tables through its view", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());

    // list a fixed key while watching jobs
    const controller: Controller = {
        name: "blind",
        mode: "follow",
        watches: [job],
        list: async () => ["blind"],
        reconcile: async () => undefined,
    };
    const loop = new ControlLoop(storage.database, [controller], {
        clock: Clock.system,
        report: () => {},
    });

    // fail the loop instead of never listing again
    await expect(loop.run(new AbortController().signal)).rejects.toEqual(
        new TypeError("blind follows tables its list never reads through its view"),
    );
});

test("follow a key again once its list names it after it failed and was dropped", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "flaky", scope: "space", runs: 1 });

    // fail the first follow and follow until stopped
    const started: string[] = [];
    const controller: Controller = {
        name: "flaky",
        mode: "follow",
        watches: [job],
        concurrency: 8,
        list: async (view) => (await view.read(job).findMany()).map((row) => row.id),
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
        clock: Clock.system,
        report: () => {},
        retry: { initialInterval: 10 },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // retry the failed follow and follow the key again once listed
    await expect.poll(() => started).toEqual(["flaky", "flaky"]);
    await database.delete(job);
    await database.insert(job).values({ id: "other", scope: "space", runs: 1 });
    await expect.poll(() => started).toEqual(["flaky", "flaky", "other"]);
    await database.insert(job).values({ id: "flaky", scope: "space", runs: 1 });
    await expect.poll(() => started).toEqual(["flaky", "flaky", "other", "flaky"]);
});

test("set the host's alarm at the earliest due or skipped key, clear it once nothing is due, and settle idle waiters, also after the loop stopped", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
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
    const clock = new VirtualClock(Date.now());
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

    // clear the alarm once the clock reached "later" again and nothing is due
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
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
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
    const loop = new ControlLoop(database, [controller], { clock: Clock.system, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await loop.idle();

    // commit changes one after another, each followed by an idle wait seeing its key reconciled
    const seen: string[][] = [];
    for (const id of ["first", "second", "third"]) {
        await database.insert(job).values({ id, scope: "space", runs: 1 });
        await loop.idle();
        seen.push([...reconciled]);
    }
    expect(seen).toEqual([["first"], ["first", "second"], ["first", "second", "third"]]);
});

test("hold idle while the follow is still keying a watched change, and settle once its key ran", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // hold the follow inside the keys of a change until released
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
    const loop = new ControlLoop(database, [controller], { clock: Clock.system, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await loop.idle();

    // commit a change and wait idle while the follow keys it
    await database.insert(job).values({ id: "changed", scope: "space", runs: 1 });
    await keying.promise;
    let isSettled = false;
    const idling = loop.idle().then(() => {
        isSettled = true;
    });
    for (let turn = 0; turn < QUIET_TURNS; turn++) {
        await yieldToEventLoop();
    }
    const isSettledWhileKeying = isSettled;

    // release the follow and settle once the key ran
    release.resolve();
    await idling;
    expect({ isSettledWhileKeying, reconciled }).toEqual({
        isSettledWhileKeying: false,
        reconciled: ["changed"],
    });
});

test("spend no statement on commits to tables no controller watches", async () => {
    const storage = await TestDatabase.create("sqlite", [job, note], { isMigrated: true });
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
    const loop = new ControlLoop(database, [controller], { clock: Clock.system, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await loop.idle();

    // commit notes one at a time, letting the loop see each, and count every statement
    const before = database.driver.state.statements;
    for (let index = 0; index < QUIET_TURNS; index++) {
        await database.insert(note).values({ id: `note-${index}`, scope: "space" });
        await yieldToEventLoop();
    }
    expect(database.driver.state.statements - before).toBe(QUIET_TURNS);
});

test("settle idle once the log moved past a quiet watched table, also after compacting its last change", async () => {
    const storage = await TestDatabase.create("sqlite", [job, note], { isMigrated: true });
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
    const loop = new ControlLoop(database, [controller], { clock: Clock.system, report: ignore });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // change the watched table once, then only another one, compacting past the watched change
    await database.insert(job).values({ id: "first", scope: "space", runs: 1 });
    await loop.idle();
    for (const id of ["one", "two", "three"]) {
        await database.insert(note).values({ id, scope: "space" });
    }
    const now = Date.now();
    await database.log.compact(now + 1, now);

    // settle with the follow at the head
    await expect(loop.idle()).resolves.toBeUndefined();
});

test("settle idle only after the key of a watched change committed while the alarm was being set", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
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

    // commit a watched change while the alarm for "later" is set, holding it until the follow keyed it
    let isChanged = false;
    const alarm = {
        setAlarm: async () => {
            if (!isChanged && reconciled.length === 1) {
                isChanged = true;
                await database.insert(job).values({ id: "changed", scope: "space", runs: 1 });
                await keyed.promise;
                await yieldToEventLoop();
            }
        },
        deleteAlarm: async () => {},
    };
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], {
        clock: Clock.system,
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
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
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
        clock: Clock.system,
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
    await expect.poll(() => reconciled, { timeout: 1000 }).toEqual(["first", "late"]);
});

test("settle an alarm at its deadline while a follow runs, leave the alarm due, and resume the follow from its recorded position after eviction", async () => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(job).values({ id: "copy", scope: "space", runs: 0 });

    // follow a growing source
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
            clock: Clock.system,
            report: ignore,
            alarm,
        });

        return { loop, stopping, running: loop.run(stopping.signal) };
    };
    const evicted = start();

    // settle the alarm at its deadline with the follow running
    await evicted.loop.idle(Date.now() + 20);
    const settled = Date.now();
    const isDue = alarms.length === 1 && nth(alarms, 0) <= settled;

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
    // run a reconciliation whose key changes when told to
    const changes: (() => void)[] = [];
    const reconciliation = {
        signal: new AbortController().signal,
        changed: () =>
            new Promise<void>((resolve) => {
                changes.push(resolve);
            }),
    };
    const change = () => changes.splice(0).forEach((resolve) => resolve());

    // abort work once a change shows it was cancelled
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
