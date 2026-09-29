import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { eq, type DatabaseConnection, type Table } from "@destack/db";
import { Condition, Expression } from "@destack/db/query";
import type { LogPosition } from "@destack/db/log";
import { Feed } from "./feed.ts";
import { Replica } from "../replica/replica.ts";
import { comment, open, openCopy, page, project, tag, task, taskTag } from "../test/fixture.ts";
import { Follower } from "./test/copy.ts";
import { ConditionAudience } from "./test/audience.ts";
import {
    AUDIENCE,
    EVERYTHING,
    FILTERS,
    LOOKUPS,
    PATHS,
    TALLIES,
    TREE,
    WINDOWS,
} from "./test/queries.ts";
import { Random } from "./test/random.ts";
import { Workload } from "./test/workload.ts";

/** The warmup writes before measuring. */
const WARMUP_WRITES = 20;

/** The queries of the snapshot budget. */
const SNAPSHOT_QUERIES = { ...FILTERS, ...WINDOWS, ...TREE, ...TALLIES, ...PATHS };

/** The tables the workload writes and subscribers copy. */
const TABLES: readonly Table[] = [project, task, comment, tag, taskTag, page];

/**
 * The most database operations a snapshot of every query set may cost.
 *
 * A snapshot reads each root page, window partition and aggregate batch once: about 65.
 */
const SNAPSHOT_OPERATIONS = 100;

/**
 * The most database operations a replica may cost to hold the same snapshot.
 *
 * A replica stages the pages and applies them in one transaction: about 5.
 */
const REPLICA_OPERATIONS = 8;

/**
 * The most database operations one write may cost a hundred subscribers of one audience, on average.
 *
 * The subscribers share one decision: well under one.
 */
const DECISION_OPERATIONS = 1;

/**
 * The most database operations one write may cost a thousand subscribers, on average.
 *
 * One shared evaluation reads the changed rows and the log once: about 6.
 */
const FAN_OUT_OPERATIONS = 10;

/** The subscribers of one space. */
const SPACE_SUBSCRIBERS = 300;

/** The distinct audiences of the space's subscribers. */
const SPACE_AUDIENCES = 50;

/**
 * The most database operations one write may cost every subscriber of a space, on average.
 *
 * The 50 audiences share each read: about 6.
 */
const SPACE_OPERATIONS = 10;

/** The rows a follower of every query set holds at scale. */
const SCALE_ROWS = 3000;

/**
 * The most database operations one write may cost a follower of every query set at scale, on average.
 *
 * Each touched node reads its changed partitions and relations in one batch.
 */
const SCALE_OPERATIONS = 12;

/** The rows one write flips in and out of every query. */
const FLIPPED_ROWS = 1500;

/** The tasks one window sorts by how many comments each has. */
const RANKED_ROWS = 10_000;

/** The ten most commented tasks. */
const DISCUSSED = {
    discussed: {
        table: task,
        scopes: ["inbox"],
        relations: {
            comments: { table: comment, on: { kind: "key", column: "taskId", parent: "id" } },
        },
        compute: { comments: Expression.rollup("count", "comments") },
        order: [{ column: "comments", direction: "desc" }],
        limit: 10,
    },
} as const;

/** The most database operations a comment may cost the window. */
const RANKED_OPERATIONS = 12;

/** The most database operations a write flipping every row may cost, at 90 keys a read. */
const FLIP_OPERATIONS = 60;

test.for(TEST_DIALECTS)(
    "snapshot and copy thousands of rows within an operation budget on %s",
    { timeout: 30_000 },
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES);
        await new Workload(new Random(3)).seed(source, 4000);

        // snapshot every query set into memory
        const snapshotOnce = async () => {
            const operations = source.driver.state.operations;
            const start = cpuTime();
            const follower = new Follower(feed, SNAPSHOT_QUERIES, TABLES, { audience: AUDIENCE });
            follower.start();
            await follower.reach(await source.log.position());
            await follower.stop();

            return {
                milliseconds: cpuTime() - start,
                operations: source.driver.state.operations - operations,
                rows: follower.copy.rows.size,
            };
        };
        await snapshotOnce();
        const { milliseconds: snapshot, operations, rows } = await snapshotOnce();

        // snapshot them through a replica
        const copyOnce = async () => {
            const client = await openCopy(dialect);
            const replica = new Replica({ name: "copy", scope: "inbox", tables: TABLES });
            const start = cpuTime();
            const applied = client.driver.state.operations;
            const replicated = new Follower(feed, SNAPSHOT_QUERIES, TABLES, {
                audience: AUDIENCE,
                replica: { replica, database: client },
            });
            replicated.start();
            await replicated.reach(await source.log.position());
            await replicated.stop();

            return {
                milliseconds: cpuTime() - start,
                operations: client.driver.state.operations - applied,
            };
        };
        await copyOnce();
        const copied = await copyOnce();

        // stay within the budgets and report the time per row
        report("snapshot CPU µs per row", (snapshot * 1000) / rows);
        report("replica CPU µs per row", (copied.milliseconds * 1000) / rows);
        expect(rows).toBeGreaterThan(2000);
        expect(operations, "operations per snapshot").toBeLessThan(SNAPSHOT_OPERATIONS);
        expect(copied.operations, "operations per replicated snapshot").toBeLessThan(
            REPLICA_OPERATIONS,
        );
    },
);

test.for(TEST_DIALECTS)(
    "decide writes for a hundred subscribers within an operation budget on %s",
    { timeout: 30_000 },
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES);
        const workload = new Workload(new Random(9));
        await workload.seed(source, 500);

        // follow filters and windows with a hundred subscribers
        const followers = Array.from(
            { length: 100 },
            () => new Follower(feed, { ...FILTERS, ...WINDOWS }, TABLES, { audience: AUDIENCE }),
        );
        for (const follower of followers) {
            follower.start();
        }
        const snapshot = await source.log.position();
        await Promise.all(followers.map((follower) => follower.reach(snapshot)));

        // time a burst of writes
        const writes = 40;
        const burst = async () => {
            for (let index = 0; index < writes; index += 1) {
                await workload.write(source);
            }
            const start = cpuTime();
            const operations = source.driver.state.operations;
            const position = await source.log.position();
            await Promise.all(followers.map((follower) => follower.reach(position)));

            return {
                milliseconds: cpuTime() - start,
                operations: (source.driver.state.operations - operations) / writes,
            };
        };
        await burst();
        const { milliseconds: elapsed, operations } = await burst();
        await Promise.all(followers.map((follower) => follower.stop()));

        // stay within the budget and report the time
        report("CPU µs per decision", (elapsed * 1000) / (followers.length * writes));
        expect(operations, "operations per write").toBeLessThan(DECISION_OPERATIONS);
    },
);

test.for(TEST_DIALECTS)(
    "resume a stateful subscriber without sending held rows again on %s",
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES);
        await new Workload(new Random(13)).seed(source, 1500);

        // follow, then reconnect from the held position
        const follower = new Follower(feed, { ...FILTERS, ...WINDOWS }, TABLES, {
            audience: AUDIENCE,
        });
        follower.start();
        await follower.reach(await source.log.position());
        const held = follower.copy.pages.length;
        await follower.reconnect();
        await follower.reach(await source.log.position());

        // rebuild the windows without rows or a snapshot
        const resumed = follower.copy.pages.slice(held);
        expect([
            resumed.some((page) => page.reset),
            resumed.flatMap((page) => page.changes),
        ]).toEqual([false, []]);
    },
);

test.for(TEST_DIALECTS)(
    "reach a thousand subscribers of one row within an operation budget on %s",
    { timeout: 30_000 },
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES, { subscribers: 1000 });
        await new Workload(new Random(21)).seed(source, 200);

        // follow with a thousand subscribers
        const controller = new AbortController();
        const reached = Array.from({ length: 1000 }, () => -1);
        let wake = () => {};
        const subscribers = reached.map(async (_, index) => {
            const pages = feed.subscribe({ ...FILTERS, ...WINDOWS }, undefined, controller.signal, {
                audience: AUDIENCE,
            });
            for await (const page of pages) {
                reached[index] = page.complete ? page.position.sequence : reached[index]!;
                wake();
            }
        });
        const reach = async (position: LogPosition) => {
            while (reached.some((sequence) => sequence < position.sequence)) {
                await new Promise<void>((resolve) => (wake = resolve));
            }
        };
        await reach(await source.log.position());

        // time a write every subscriber holds
        const { milliseconds, operations } = await measure(
            source,
            40,
            async (index) => {
                await source
                    .update(task)
                    .set({ rank: index % 10 })
                    .where(eq(task.id, "s1"));
            },
            reach,
        );
        controller.abort();
        await Promise.all(subscribers);

        // stay within the budget and report the time
        report("CPU ms per write", milliseconds);
        expect(operations, "operations per write").toBeLessThan(FAN_OUT_OPERATIONS);
    },
);

test.for(TEST_DIALECTS)(
    "flip fifteen hundred rows through a lookup within an operation budget on %s",
    { timeout: 60_000 },
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES);

        // follow the tasks of a named project
        await source.insert(project).values({ id: "p0", scope: "inbox", name: "a" });
        await source.insert(task).values(
            Array.from({ length: FLIPPED_ROWS }, (_, index) => ({
                id: `t${index}`,
                scope: "inbox",
                projectId: "p0",
                state: "open",
                rank: index % 10,
                points: 1,
                title: "work",
                isSecret: false,
            })),
        );
        const follower = new Follower(feed, LOOKUPS, TABLES, { audience: AUDIENCE });
        follower.start();
        await follower.reach(await source.log.position());

        // rename the project back and forth
        const { milliseconds, operations } = await measure(
            source,
            6,
            async (index) => {
                await source
                    .update(project)
                    .set({ name: index % 2 === 0 ? "x" : "a" })
                    .where(eq(project.id, "p0"));
            },
            (position) => follower.reach(position),
        );
        await follower.stop();

        // stay within the budget and report the time
        report("CPU ms per flip", milliseconds);
        expect(operations, "operations per flip").toBeLessThan(FLIP_OPERATIONS);
    },
);

test.for(TEST_DIALECTS)(
    "follow every query set at scale within an operation budget on %s",
    { timeout: 60_000 },
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES);
        const workload = new Workload(new Random(31));
        await workload.seed(source, SCALE_ROWS);

        // follow every query set
        const follower = new Follower(feed, EVERYTHING, TABLES, { audience: AUDIENCE });
        follower.start();
        await follower.reach(await source.log.position());

        // time single writes
        const { milliseconds, operations } = await measure(
            source,
            48,
            () => workload.write(source),
            (position) => follower.reach(position),
        );
        await follower.stop();

        // stay within the budget and report the time
        report("CPU ms per write", milliseconds);
        expect(operations, "operations per write").toBeLessThan(SCALE_OPERATIONS);
    },
);

test.for(TEST_DIALECTS)(
    "reach the subscribers of a space with distinct access within an operation budget on %s",
    { timeout: 60_000 },
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES, { subscribers: SPACE_SUBSCRIBERS });
        const workload = new Workload(new Random(41));
        await workload.seed(source, 500);

        // follow as subscribers of distinct audiences
        const audiences = Array.from(
            { length: SPACE_AUDIENCES },
            (_, index) =>
                new ConditionAudience(new Map([[task as Table, Condition.ne("id", `s${index}`)]])),
        );
        const followers = Array.from(
            { length: SPACE_SUBSCRIBERS },
            (_, index) =>
                new Follower(feed, { ...FILTERS, ...WINDOWS }, TABLES, {
                    audience: audiences[index % SPACE_AUDIENCES]!,
                }),
        );
        for (const follower of followers) {
            follower.start();
        }
        const snapshot = await source.log.position();
        await Promise.all(followers.map((follower) => follower.reach(snapshot)));

        // time single writes
        const { milliseconds, operations } = await measure(
            source,
            40,
            () => workload.write(source),
            async (position) => {
                await Promise.all(followers.map((follower) => follower.reach(position)));
            },
        );
        await Promise.all(followers.map((follower) => follower.stop()));

        // stay within the budget and report the time
        report("CPU ms per write", milliseconds);
        expect(operations, "operations per write").toBeLessThan(SPACE_OPERATIONS);
    },
);

test.for(TEST_DIALECTS)(
    "reorder ten thousand tasks by a rollup within an operation budget on %s",
    { timeout: 60_000 },
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES);

        // file ten thousand tasks and follow the ten most commented
        for (let start = 0; start < RANKED_ROWS; start += 500) {
            await source.insert(task).values(
                Array.from({ length: 500 }, (_, index) => ({
                    id: `t${start + index}`,
                    scope: "inbox",
                    state: "open",
                    rank: 0,
                    isSecret: false,
                })),
            );
        }
        const follower = new Follower(feed, DISCUSSED, TABLES, { audience: AUDIENCE });
        follower.start();
        await follower.reach(await source.log.position());

        // comment on random tasks
        const random = new Random(17);
        const { milliseconds, operations } = await measure(
            source,
            60,
            async (index) => {
                await source.insert(comment).values({
                    id: `c${index}`,
                    scope: "inbox",
                    taskId: `t${random.integer(40)}`,
                    position: index,
                });
            },
            (position) => follower.reach(position),
        );
        await follower.stop();

        // stay within the budget and report the time
        report("CPU ms per comment", milliseconds);
        expect(operations, "operations per comment").toBeLessThan(RANKED_OPERATIONS);
    },
);

/** Run a scenario's writes after warmup, returning the average CPU time and operations per write. */
async function measure(
    source: DatabaseConnection,
    writes: number,
    write: (index: number) => Promise<unknown>,
    reach: (position: LogPosition) => Promise<unknown>,
): Promise<{ readonly milliseconds: number; readonly operations: number }> {
    // warm up
    for (let index = 0; index < WARMUP_WRITES; index += 1) {
        await write(index);
        await reach(await source.log.position());
    }

    // time each write until its subscribers hold it
    let elapsed = 0;
    const operations = source.driver.state.operations;
    for (let index = WARMUP_WRITES; index < WARMUP_WRITES + writes; index += 1) {
        await write(index);
        const committed = cpuTime();
        await reach(await source.log.position());
        elapsed += cpuTime() - committed;
    }

    return {
        milliseconds: elapsed / writes,
        operations: (source.driver.state.operations - operations) / writes,
    };
}

/** Read the process CPU time, in milliseconds. */
function cpuTime(): number {
    const { user, system } = process.cpuUsage();

    return (user + system) / 1000;
}

/** Report a measured cost beside the test result. */
function report(label: string, value: number): void {
    console.info(`${expect.getState().currentTestName}: ${label} ${value.toFixed(2)}`);
}
