import { expect, onTestFinished, test, type TestContext } from "@destack/test";
import { defineTable, eq, integer, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ControlLoop, type Controller } from "./control.ts";
import { Clock } from "@destack/environment";

/** The keys one burst reconciles. */
const KEYS = 20;

/** The reads each reconciliation makes, which a synchronous database answers without yielding. */
const READS = 20;

/** The synchronous work each reconciliation does beside its reads, such as decoding, in ms. */
const WORK_MILLISECONDS = 2;

/** The interval of the timer that probes the event loop, in milliseconds. */
const PROBE_MILLISECONDS = 1;

/** The jobs a controller reconciles. */
const job = defineTable("controller_bench_job", {
    /** The job's name. */
    id: text("id").primaryKey(),
    /** The desired number of runs. */
    runs: integer("runs").notNull(),
});

test("run timers between the reconciliations of a synchronous database, one key per turn", async ({
    annotate,
}) => {
    const storage = await TestDatabase.create("sqlite", [job], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database
        .insert(job)
        .values(Array.from({ length: KEYS }, (_, index) => ({ id: `job-${index}`, runs: 1 })));

    // record the reconciliations and the probe's ticks in the order they run
    const events: ("reconcile" | "tick")[] = [];
    const done = Promise.withResolvers<void>();
    const controller: Controller = {
        name: "job",
        list: async () => Array.from({ length: KEYS }, (_, index) => `job-${index}`),
        reconcile: async (key) => {
            // read the job a few times and work on it without yielding
            for (let read = 0; read < READS; read++) {
                await database.select().from(job).where(eq(job.id, key));
            }
            const end = performance.now() + WORK_MILLISECONDS;
            while (performance.now() < end) {
                // occupy the thread as decoding does
            }

            // finish the burst at the last key
            events.push("reconcile");
            if (events.filter((event) => event === "reconcile").length === KEYS) {
                done.resolve();
            }

            return undefined;
        },
    };

    // probe the event loop while the loop reconciles every key
    let last = performance.now();
    let longest = 0;
    const probe = setInterval(() => {
        const now = performance.now();
        longest = Math.max(longest, now - last);
        last = now;
        events.push("tick");
    }, PROBE_MILLISECONDS);
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [controller], { clock: Clock.system, report: fail });
    const start = performance.now();
    const running = loop.run(stopping.signal);
    await done.promise;
    const elapsed = performance.now() - start;
    longest = Math.max(longest, performance.now() - last);
    clearInterval(probe);
    stopping.abort();
    await running;

    // tick between every two reconciliations but the first two, which share the turn the listing woke
    const reconciled = events.flatMap((event, index) => (event === "reconcile" ? [index] : []));
    const interleaved = reconciled
        .slice(1)
        .filter((at, index) => events.slice(reconciled[index], at).includes("tick"));
    await report(annotate, "longest event loop block ms", longest);
    await report(annotate, "reconciliation ms", elapsed / KEYS);
    expect(reconciled.length).toBe(KEYS);
    expect(interleaved.length).toBe(KEYS - 2);
});

/** Fail on a reported reconciliation failure. */
function fail(_controller: Controller, key: string, error: unknown): void {
    throw new Error(`reconciling ${key} failed`, { cause: error });
}

/** Report a measured cost beside the test result. */
async function report(annotate: TestContext["annotate"], label: string, value: number) {
    await annotate(`${label} ${value.toFixed(2)}`, "cost");
}
