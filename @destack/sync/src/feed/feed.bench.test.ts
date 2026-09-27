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

/** The writes a scenario runs before measuring, so that measured writes run compiled code. */
const WARMUP_WRITES = 20;

/** The queries a snapshot's per-row budget was measured on: filters, windows, a tree, tallies and paths. */
const SNAPSHOT_QUERIES = { ...FILTERS, ...WINDOWS, ...TREE, ...TALLIES, ...PATHS };

/** The tables the workload writes and subscribers copy. */
const TABLES: readonly Table[] = [project, task, comment, tag, taskTag, page];

/**
 * The most database operations a snapshot of every query set may cost.
 *
 * It reads a page of each root's rows, each window partition and each aggregate batch once, with a head read each: about 65, with room for half again.
 */
const SNAPSHOT_OPERATIONS = 100;

/**
 * The most database operations a replica may cost to hold the same snapshot.
 *
 * It stages the pages and applies them in one completing transaction: about 5.
 */
const REPLICA_OPERATIONS = 8;

/**
 * The most database operations one committed write may cost a hundred subscribers of one audience, on average.
 *
 * The subscribers share the write's decision, so it reads what it touches once for all of them: well under one.
 */
const DECISION_OPERATIONS = 1;

/**
 * The most database operations a committed write every subscriber follows may cost to reach a thousand of them, on average.
 *
 * One shared evaluation decides the write, reading the changed rows and the log once: about 6.
 */
const FAN_OUT_OPERATIONS = 10;

/** The subscribers of one space. */
const SPACE_SUBSCRIBERS = 300;

/** The distinct audiences the space's subscribers' access forms. */
const SPACE_AUDIENCES = 50;

/**
 * The most database operations one committed write may cost every subscriber of a space, on average.
 *
 * The 50 audiences share each read of a position, so the write reads what it touches about once: about 6.
 */
const SPACE_OPERATIONS = 10;

/** The rows a follower of every query set holds at scale, a few projects with hundreds of tasks each. */
const SCALE_ROWS = 3000;

/**
 * The most database operations one committed write may cost a follower of every query set at scale, on average.
 *
 * Each node a write touches reads its changed partitions and relations in one batch.
 */
const SCALE_OPERATIONS = 12;

/** The rows one write flips in and out of every query. */
const FLIPPED_ROWS = 1500;

/** The tasks one window sorts by how many comments each has. */
const RANKED_ROWS = 10_000;

/** The ten most discussed tasks, sorted by a rollup of their comments over every task. */
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

/** The most database operations a comment may cost the window, whatever the number of tasks it sorts. */
const RANKED_OPERATIONS = 12;

/** The most database operations a write flipping every row may cost: a batch of the flipped rows per node, 90 keys a read. */
const FLIP_OPERATIONS = 60;

test.for(TEST_DIALECTS)(
    "snapshot and copy thousands of rows within an operation budget on %s",
    { timeout: 30_000 },
    async (dialect) => {
        const source = await open(dialect);
        const feed = new Feed(source, TABLES);
        await new Workload(new Random(3)).seed(source, 4000);

        // snapshot every query set into memory, once to warm up and once measured
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

        // snapshot them through a replica into a second database, once to warm up and once measured
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

        // stay within the operation budgets over enough rows to page, reporting the time per row
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

        // follow filters and windows with a hundred subscribers, each holding its snapshot
        const followers = Array.from(
            { length: 100 },
            () => new Follower(feed, { ...FILTERS, ...WINDOWS }, TABLES, { audience: AUDIENCE }),
        );
        for (const follower of followers) {
            follower.start();
        }
        const snapshot = await source.log.position();
        await Promise.all(followers.map((follower) => follower.reach(snapshot)));

        // write a burst, and time until every subscriber holds it, once to warm up and once measured
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

        // stay within the operations per write, reporting the time per subscriber and write
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

        // follow filters and windows, then reconnect from the held position
        const follower = new Follower(feed, { ...FILTERS, ...WINDOWS }, TABLES, {
            audience: AUDIENCE,
        });
        follower.start();
        await follower.reach(await source.log.position());
        const held = follower.copy.pages.length;
        await follower.reconnect();
        await follower.reach(await source.log.position());

        // rebuild windows as the subscriber held them, sending no rows and no snapshot
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

        // follow the same queries with a thousand subscribers, each noting the last position it holds
        const controller = new AbortController();
        const reached = Array.from({ length: 1000 }, () => -1);
        const subscribers = reached.map(async (_, index) => {
            const pages = feed.subscribe({ ...FILTERS, ...WINDOWS }, undefined, controller.signal, {
                audience: AUDIENCE,
            });
            for await (const page of pages) {
                reached[index] = page.complete ? page.position.sequence : reached[index]!;
            }
        });
        const reach = async (position: LogPosition) => {
            while (reached.some((sequence) => sequence < position.sequence)) {
                await new Promise((resolve) => setTimeout(resolve, 1));
            }
        };
        await reach(await source.log.position());

        // change one row every subscriber holds, timing each write from its commit until every subscriber holds it
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

        // stay within the operations per write, reporting the time
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

        // file every task in one project, and follow the tasks by the project's name
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

        // rename the project back and forth, flipping every task out of the named tasks and back, timing each write until the subscriber holds it
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

        // stay within the operations per write, reporting the time
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

        // follow every query set, holding its snapshot
        const follower = new Follower(feed, EVERYTHING, TABLES, { audience: AUDIENCE });
        follower.start();
        await follower.reach(await source.log.position());

        // write one row at a time, timing each write from its commit until the follower holds it
        const { milliseconds, operations } = await measure(
            source,
            48,
            () => workload.write(source),
            (position) => follower.reach(position),
        );
        await follower.stop();

        // stay within the operations per write, the workload's own writes among them, reporting the time
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

        // follow filters and windows as subscribers of distinct audiences, each hiding one task of its own
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

        // write one row at a time, timing each write from its commit until every subscriber holds it
        const { milliseconds, operations } = await measure(
            source,
            40,
            () => workload.write(source),
            async (position) => {
                await Promise.all(followers.map((follower) => follower.reach(position)));
            },
        );
        await Promise.all(followers.map((follower) => follower.stop()));

        // stay within the operations per write, reporting the time
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

        // file ten thousand tasks, then follow the ten with the most comments
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

        // comment on random tasks, moving them into and among the ten, timing each write until the subscriber holds it
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

        // stay within the operations per write, the comments among them, reporting the time
        report("CPU ms per comment", milliseconds);
        expect(operations, "operations per comment").toBeLessThan(RANKED_OPERATIONS);
    },
);

/**
 * Run a scenario's writes after warmup writes, returning the CPU time and database operations each measured write spent until its subscribers held it, on average.
 *
 * The warmup lets measured writes run compiled code, as a long running server does.
 */
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

    // time each write from its commit until its subscribers hold it
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

/** Read the CPU time the process spent so far, in milliseconds, which a loaded machine stretches less than wall time. */
function cpuTime(): number {
    const { user, system } = process.cpuUsage();

    return (user + system) / 1000;
}

/** Report a measured cost beside the test's result, which tracks it without failing on a loaded machine. */
function report(label: string, value: number): void {
    console.info(`${expect.getState().currentTestName}: ${label} ${value.toFixed(2)}`);
}
