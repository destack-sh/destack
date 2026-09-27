import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import {
    encodeRow,
    Key,
    TABLE,
    type DatabaseConnection,
    type Dialect,
    type Table,
} from "@destack/db";
import { Feed } from "./feed.ts";
import { PAGE_ROWS } from "../dataflow/selection.ts";
import type { Query } from "../query/query.ts";
import { Replica, replicaResult } from "../replica/replica.ts";
import { comment, open, openCopy, page, project, tag, task, taskTag } from "../test/fixture.ts";
import { Follower } from "./test/copy.ts";
import { evaluate, type Holding } from "./test/oracle.ts";
import { AUDIENCE, EVERYTHING, FILTERS, QUERY_SETS, TREE, WINDOWS } from "./test/queries.ts";
import { Random } from "./test/random.ts";
import { Workload } from "./test/workload.ts";

/** The tables the workload writes and subscribers copy. */
const TABLES: readonly Table[] = [project, task, comment, tag, taskTag, page];

/** The writes each burst makes before the subscriber must hold them. */
const WRITES = 8;

/** The longest a run may take, in milliseconds. */
const RUN_MILLISECONDS = 15_000;

/**
 * The longest the run at scale may take, in milliseconds.
 *
 * It copies about 2900 rows through a replica four times, a snapshot and three rebuilds of about 3 s each on PostgreSQL under load.
 */
const SCALE_RUN_MILLISECONDS = 30_000;

/** One random run: queries followed through bursts of writes, reconnecting, and moving to other queries, on a cadence. */
interface Run {
    /** The queries followed first. */
    readonly queries: Readonly<Record<string, Query>>;
    /** The seed of the writes. */
    readonly seed: number;
    /** The bursts of writes. */
    readonly bursts: number;
    /** The bursts after which the subscriber reconnects from its position. */
    readonly reconnect: number;
    /** The queries the subscriber moves to halfway, when given. */
    readonly reshape?: Readonly<Record<string, Query>>;
    /** Whether the pages pass through a replica into a second database. */
    readonly isReplicated?: boolean;
    /** The tasks and comments inserted before following, which pages several snapshot pages deep. */
    readonly seedRows?: number;
}

/** The run at scale: every query set over thousands of rows through a replica, reconnecting every other burst. */
const SCALE_RUN: Run = {
    queries: EVERYTHING,
    seed: 31,
    bursts: 6,
    reconnect: 2,
    seedRows: 3000,
    isReplicated: true,
};

/** The runs, by name: each query set with two seeds, a replicated run, and moves between stateless and stateful queries. */
const RUNS: Readonly<Record<string, Run>> = {
    ...Object.fromEntries(
        Object.entries(QUERY_SETS).flatMap(([name, queries]) => [
            [`${name} (seed 5)`, { queries, seed: 5, bursts: 20, reconnect: 4 }],
            [`${name} (seed 11)`, { queries, seed: 11, bursts: 20, reconnect: 7 }],
        ]),
    ),
    "everything through a replica": {
        queries: EVERYTHING,
        seed: 17,
        bursts: 20,
        reconnect: 6,
        isReplicated: true,
    },
    "filters moved to a tree": {
        queries: FILTERS,
        seed: 23,
        bursts: 16,
        reconnect: 5,
        reshape: TREE,
    },
    "windows moved to filters": {
        queries: WINDOWS,
        seed: 29,
        bursts: 16,
        reconnect: 5,
        reshape: FILTERS,
    },
};

test.for(
    TEST_DIALECTS.flatMap((dialect) => Object.keys(RUNS).map((name) => [name, dialect] as const)),
)(
    "hold exactly what %s selects through random writes on %s",
    { timeout: RUN_MILLISECONDS },
    ([name, dialect]) => holdExactly(RUNS[name]!, dialect),
);

test.for(TEST_DIALECTS)(
    "hold exactly what everything at scale selects through random writes on %s",
    { timeout: SCALE_RUN_MILLISECONDS },
    (dialect) => holdExactly(SCALE_RUN, dialect),
);

/** Follow a run's queries through its bursts of random writes, holding exactly what an oracle selects after each. */
async function holdExactly(run: Run, dialect: Dialect): Promise<void> {
    const source = await open(dialect);
    const feed = new Feed(source, TABLES);
    const workload = new Workload(new Random(run.seed));
    if (run.seedRows !== undefined) {
        await workload.seed(source, run.seedRows);
    }

    // follow the queries, through a replica into a second database when replicated
    const client = run.isReplicated ? await openCopy(dialect) : undefined;
    const replica = new Replica({ name: "copy", scope: "inbox", tables: TABLES });
    const follower = new Follower(feed, run.queries, TABLES, {
        audience: AUDIENCE,
        ...(client === undefined ? {} : { replica: { replica, database: client } }),
    });
    follower.start();

    let queries = run.queries;
    for (let burst = 0; burst < run.bursts; burst += 1) {
        // write a burst, and wait until the subscriber holds it
        for (let index = 0; index < WRITES; index += 1) {
            await workload.write(source);
        }
        await follower.reach(await source.log.position());

        // hold more than a page of rows when seeded, so that snapshots and rebuilds page
        if (run.seedRows !== undefined) {
            expect(follower.copy.rows.size).toBeGreaterThan(PAGE_ROWS);
        }

        // hold exactly what the queries select in memory, in the copy and in the replica
        const expected = holding(await evaluate(source, queries, AUDIENCE));
        expect([burst, ...holding(follower.copy)]).toEqual([burst, ...expected]);
        if (client !== undefined) {
            expect([burst, ...holding(await replicated(client))]).toEqual([burst, ...expected]);
        }

        // move to other queries halfway, and reconnect on the cadence
        if (run.reshape !== undefined && burst === Math.floor(run.bursts / 2)) {
            queries = run.reshape;
            await follower.reconnect(queries);
        } else if (burst % run.reconnect === run.reconnect - 1) {
            await follower.reconnect();
        }
    }
    await follower.stop();
}

/** Present a holding for comparison, rows and groups sorted by key. */
function holding(held: Holding): [Record<string, unknown>, Record<string, unknown>] {
    return [
        Object.fromEntries([...held.rows].sort(([left], [right]) => (left < right ? -1 : 1))),
        Object.fromEntries([...held.results].sort(([left], [right]) => (left < right ? -1 : 1))),
    ];
}

/** Read what a replica database holds: its copied rows and aggregate groups. */
async function replicated(database: DatabaseConnection): Promise<Holding> {
    // read every copied row as JSON of its logged columns
    const rows = new Map<string, Record<string, unknown>>();
    for (const table of TABLES) {
        for (const row of (await database.select().from(table as never)) as Record<
            string,
            unknown
        >[]) {
            const logged = Object.keys(table[TABLE].logged).map((name) => [name, row[name]]);
            rows.set(Key.name(table, row), encodeRow(table, Object.fromEntries(logged)));
        }
    }

    // read every copied group by query and group
    const results = new Map(
        (await database.select().from(replicaResult)).map((row) => [
            JSON.stringify(row.query) + row.group,
            row.values,
        ]),
    );

    return { rows, results };
}
