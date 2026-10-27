import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { Key, TABLE, type DatabaseConnection, type Table } from "@destack/db";
import { Feed } from "../feed.ts";
import type { Query } from "../../query/query.ts";
import { Replica, replicaResult } from "../../replica/replica.ts";
import {
    comment,
    open,
    openReplica,
    page,
    project,
    tag,
    task,
    taskTag,
    START,
} from "../../test/fixture.ts";
import { Follower } from "./replica.ts";
import { evaluate, type Contents } from "./oracle.ts";
import type { ConditionAudience } from "./audience.ts";
import { AUDIENCE } from "./query.ts";
import { Random } from "./random.ts";
import { Workload } from "./workload.ts";

/** The tables the workload writes and subscribers replicate. */
const TABLES: readonly Table[] = [project, task, comment, tag, taskTag, page];

/** The writes of each burst. */
const WRITES = 8;

/** The longest a run may take, in milliseconds. */
const RUN_MILLISECONDS = 15_000;

/** The longest the run at scale may take, in milliseconds. */
const SCALE_RUN_MILLISECONDS = 30_000;

/** One random run of queries through bursts of writes, reconnects and reshapes. */
export interface Run {
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
    /** Whether the subscriber moves to them on its open stream, else by reconnecting. */
    readonly isReshapedInPlace?: boolean;
    /** Whether the pages pass through a replica into a second database. */
    readonly isReplicated?: boolean;
    /** The tasks and comments inserted before following. */
    readonly seedRows?: number;
    /** The most rows one page reads, a few dozen to cross pages with few rows. */
    readonly pageSize?: number;
    /** The deciding audience. */
    readonly audience?: ConditionAudience;
}

/** Run each named run and match exactly what an oracle selects. */
export function followRuns(runs: Readonly<Record<string, Run>>): void {
    test.for(Object.entries(runs))(
        "match exactly what %s selects through random writes",
        { timeout: RUN_MILLISECONDS },
        ([, run]) => matchExactly(run),
    );
}

/** Run a run at scale. */
export function followAtScale(name: string, run: Run): void {
    test(
        `match exactly what ${name} selects through random writes`,
        { timeout: SCALE_RUN_MILLISECONDS },
        () => matchExactly(run),
    );
}

/** Build the seeded runs of some query sets. */
export function seededRuns(
    sets: Readonly<Record<string, Readonly<Record<string, Query>>>>,
): Record<string, Run> {
    return Object.fromEntries(
        Object.entries(sets).flatMap(([name, queries]) => [
            [`${name} (seed 5)`, { queries, seed: 5, bursts: 20, reconnect: 4 }],
            [`${name} (seed 11)`, { queries, seed: 11, bursts: 20, reconnect: 7 }],
        ]),
    );
}

/** Follow a run's queries through random writes and match exactly what an oracle selects. */
async function matchExactly(run: Run): Promise<void> {
    // open the source and seed its rows on the run's one clock
    const simulation = new Simulation({ seed: 1, start: START });
    const source = await open({ simulation });
    const feed = new Feed(
        source,
        TABLES,
        run.pageSize === undefined ? {} : { pageSize: run.pageSize },
    );
    const workload = new Workload(new Random(run.seed));
    if (run.seedRows !== undefined) {
        await workload.seed(source, run.seedRows);
    }

    // follow the queries
    const client = run.isReplicated === true ? await openReplica({ simulation }) : undefined;
    const replica = new Replica({ name: "replica", scope: "inbox", tables: TABLES });
    const audience = run.audience ?? AUDIENCE;
    const follower = new Follower(feed, run.queries, TABLES, {
        audience,
        ...(client === undefined ? {} : { replica: { replica, database: client } }),
    });
    follower.start();

    // write bursts and match the oracle after each
    let queries = run.queries;
    for (let burst = 0; burst < run.bursts; burst += 1) {
        // write a burst and wait for it
        for (let index = 0; index < WRITES; index += 1) {
            await workload.write(source);
        }
        await follower.wait(await source.log.position());

        // seed more than a page of rows
        if (run.seedRows !== undefined) {
            expect(follower.replica.rows.size).toBeGreaterThan(feed.pageSize);
        }

        // match exactly the oracle's selection
        const expected = sorted(await evaluate(source, queries, audience));
        expect([burst, ...sorted(follower.replica)]).toEqual([burst, ...expected]);
        if (client !== undefined) {
            expect([burst, ...sorted(await replicated(client))]).toEqual([burst, ...expected]);
        }

        // reshape halfway and reconnect on the cadence
        if (run.reshape !== undefined && burst === Math.floor(run.bursts / 2)) {
            queries = run.reshape;
            if (run.isReshapedInPlace === true) {
                follower.reshape(queries);
            } else {
                await follower.reconnect(queries);
            }
        } else if (burst % run.reconnect === run.reconnect - 1) {
            await follower.reconnect();
        }
    }
    await follower.stop();
}

/** Sort some contents for comparison. */
function sorted(contents: Contents): [Record<string, unknown>, Record<string, unknown>] {
    return [
        Object.fromEntries(
            [...contents.rows].toSorted(([left], [right]) => (left < right ? -1 : 1)),
        ),
        Object.fromEntries(
            [...contents.results].toSorted(([left], [right]) => (left < right ? -1 : 1)),
        ),
    ];
}

/** Read what a replica database has. */
async function replicated(database: DatabaseConnection): Promise<Contents> {
    // read every replicated row
    const rows = new Map<string, Record<string, unknown>>();
    for (const table of TABLES) {
        for (const row of await database.select().from(table)) {
            rows.set(Key.name(table, row), table[TABLE].encodeLogged(row, []));
        }
    }

    // read every replicated group
    const results = new Map(
        (await database.select().from(replicaResult)).map((row) => [
            JSON.stringify(row.query) + row.group,
            row.values,
        ]),
    );

    return { rows, results };
}
