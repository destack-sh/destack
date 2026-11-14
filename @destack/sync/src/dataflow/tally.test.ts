import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { eq } from "@destack/db";
import { Feed } from "../feed/feed.ts";
import type { Query } from "../query/query.ts";
import { open, task, until, START } from "../test/fixture.ts";
import { testWorldOptions } from "@destack/test/world";

/** The distinct ranks of the one measured group. */
const RANKS = 40;

test("measure a group of many distinct values as its extreme, and read its values once at its first lost extreme", async () => {
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: START,
    });
    // store one group of tasks with distinct ranks, the highest measured
    const database = await open({ simulation });
    await database.transaction(
        async (transaction) =>
            await transaction.insert(task).values(
                Array.from({ length: RANKS }, (_, rank) => ({
                    id: `t${rank}`,
                    scope: "inbox",
                    projectId: null,
                    state: "open",
                    rank,
                    points: null,
                    title: null,
                    isSecret: false,
                })),
            ),
    );
    const feed = new Feed(database, [task], {});
    const query: Query = {
        table: task,
        scopes: ["inbox"],
        aggregate: {
            groupBy: ["state"],
            values: { high: { function: "max", column: "rank" } },
        },
    };
    const pages = feed.subscribe({ query }, undefined, AbortSignal.timeout(5000));
    await until(pages, (page) => page.complete);
    const dataflow = () => feed.inspect().evaluations[0]?.dataflows[0];

    // read the runs ended since the last read, from the snapshot's on, each as its changes and statements
    const runs = () =>
        simulation.telemetry.spans.filter((span) =>
            ["destack.dataflow.hydrate", "destack.dataflow.step"].includes(span.name),
        );
    let seen = 0;
    const ended = () => {
        const fresh = runs().slice(seen);
        seen += fresh.length;

        return fresh.map((span) => [
            span.attributes["destack.dataflow.changed"],
            span.attributes["db.client.operation.count"] ?? 0,
        ]);
    };

    // delete a task, listing each run's changes and statements until its page has the new extreme
    const remove = async (rank: number) => {
        await database.transaction(
            async (transaction) => await transaction.delete(task).where(eq(task.id, `t${rank}`)),
        );
        const [page] = (await until(pages, (read) => read.complete)).slice(-1);

        return {
            runs: ended(),
            high: page?.results?.[0]?.values?.["high"],
        };
    };

    // delete a counted value, its run finishing the snapshot's, then the extreme twice
    const removed = await remove(10);
    const size = dataflow()?.size;
    const first = await remove(RANKS - 1);
    const second = await remove(RANKS - 2);
    expect({
        size,
        counted: dataflow()?.size,
        // read the group's extreme at the snapshot, its values at the first loss, and nothing else
        runs: [removed.runs, first.runs, second.runs],
        high: [first.high, second.high],
    }).toEqual({
        size: 1,
        counted: 1 + RANKS - 3,
        runs: [
            [
                [0, 1],
                [1, 0],
            ],
            [[1, 1]],
            [[1, 0]],
        ],
        high: [RANKS - 2, RANKS - 3],
    });
});
