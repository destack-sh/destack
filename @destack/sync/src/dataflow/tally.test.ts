import { expect, test } from "@destack/test";
import { eq } from "@destack/db";
import { Feed } from "../feed/feed.ts";
import type { Query } from "../query/query.ts";
import type { RunCost } from "./run.ts";
import { open, task, until } from "../test/fixture.ts";
import { Clock } from "@destack/environment";

/** The distinct ranks of the one measured group. */
const RANKS = 40;

test("measure a group of many distinct values as its extreme, and read its values once at its first lost extreme", async () => {
    // store one group of tasks with distinct ranks, the highest measured
    const database = await open();
    await database.insert(task).values(
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
    );
    const costs: RunCost[] = [];
    const feed = new Feed(database, [task], {
        clock: Clock.system,
        observe: (cost) => costs.push(cost),
    });
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
    const dataflow = () => feed.inspect().evaluations[0]?.dataflow;

    // delete a task, listing each run's changes and operations until its page has the new extreme
    const remove = async (rank: number) => {
        await database.delete(task).where(eq(task.id, `t${rank}`));
        const [page] = (await until(pages, (read) => read.complete)).slice(-1);

        return {
            runs: costs.splice(0).map((cost) => [cost.changes, cost.operations]),
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
