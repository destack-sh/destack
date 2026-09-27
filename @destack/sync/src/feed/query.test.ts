import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { eq, sql, type Table } from "@destack/db";
import { Condition, Expression } from "@destack/db/query";
import type { Change } from "@destack/db/log";
import { Feed } from "./feed.ts";
import { EVERYONE, type Audience } from "./audience.ts";
import type { QueryPage } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import {
    comment,
    note,
    open,
    page,
    project,
    tag,
    task,
    taskTag,
    take,
    until,
} from "../test/fixture.ts";
import { AUDIENCE } from "./test/queries.ts";

/** The tables the feed serves. */
const TABLES: readonly Table[] = [project, task, comment, tag, taskTag, page];

/** A task of a project in the followed folder. */
const TASK = {
    id: "t1",
    scope: "inbox",
    projectId: "p1",
    state: "open",
    rank: 1,
    points: 2,
    title: "write",
    isSecret: false,
};

/** Subscribe and read the first page. */
async function first(
    feed: Feed,
    queries: Readonly<Record<string, Query>>,
    audience: Audience = EVERYONE,
) {
    const [page] = await take(
        feed.subscribe(queries, undefined, AbortSignal.timeout(5000), { audience }),
        (entry) => entry.complete,
    );

    return page!;
}

test.for(TEST_DIALECTS)(
    "refuse queries a subscription cannot keep exactly on %s",
    async (dialect) => {
        const feed = new Feed(await open(dialect), TABLES);
        const refusal = async (query: Query, audience: Audience = EVERYONE) => {
            const pages = feed.subscribe({ query }, undefined, AbortSignal.timeout(5000), {
                audience,
            });
            const error = await pages.next().then(
                () => undefined,
                (thrown: unknown) => (thrown as Error).message,
            );

            return error;
        };

        // refuse concealable, unordered, unlogged and misshapen reads with their reasons
        expect(
            await Promise.all([
                refusal(
                    { table: task, scopes: ["inbox"], where: Condition.eq("title", "write") },
                    AUDIENCE,
                ),
                refusal({
                    table: task,
                    scopes: ["inbox"],
                    order: [{ column: "missing", direction: "asc" }],
                }),
                refusal({ table: task, scopes: ["inbox"], limit: 0 }),
                refusal({
                    table: task,
                    scopes: ["inbox"],
                    aggregate: { values: { total: { function: "sum", column: "state" } } },
                }),
                refusal({ table: note, scopes: ["inbox"], where: Condition.eq("labels", "draft") }),
                refusal({ table: note, scopes: ["inbox"] }),
                refusal({
                    table: task,
                    scopes: ["inbox"],
                    limit: 2,
                    aggregate: { values: { tasks: { function: "count" } } },
                }),
                refusal(
                    {
                        table: task,
                        scopes: ["inbox"],
                        compute: { label: Expression.column("title") },
                        order: [{ column: "label", direction: "asc" }],
                    },
                    AUDIENCE,
                ),
                refusal({
                    table: task,
                    scopes: ["inbox"],
                    compute: { rank: Expression.literal(1) },
                }),
                refusal({
                    table: task,
                    scopes: ["inbox"],
                    compute: { label: Expression.column("title") },
                    aggregate: { values: { total: { function: "avg", column: "label" } } },
                }),
                refusal({
                    table: project,
                    scopes: ["inbox"],
                    relations: {
                        tasks: {
                            table: task,
                            on: { kind: "key", column: "projectId", parent: "id" },
                        },
                    },
                    compute: { spent: Expression.rollup("sum", "tasks", "title") },
                }),
                refusal({
                    table: task,
                    scopes: ["inbox"],
                    relations: {
                        tasks: {
                            table: task,
                            on: { kind: "key", column: "projectId", parent: "projectId" },
                        },
                    },
                    compute: { peers: Expression.rollup("count", "tasks") },
                }),
                refusal({
                    table: task,
                    scopes: ["inbox"],
                    compute: { peers: Expression.rollup("count", "tasks") },
                }),
            ]),
        ).toEqual([
            "query query reads concealable columns of task: title",
            "task has no column missing",
            "query limit must be a positive integer: query",
            "measure total adds up a non-numeric column",
            "note.labels is a json column",
            "feed does not read note",
            "aggregate query holds no rows to order or include: query",
            "query query reads concealable columns of task: title",
            "computed value rank shadows a column of task",
            "measure total adds up a non-numeric column",
            "measure sum(title) adds up a non-numeric column",
            "rollup tasks of query measures another table",
            "relation tasks is not declared by query",
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "conceal the columns an audience hides and name them on %s",
    async (dialect) => {
        const database = await open(dialect);
        await database.insert(task).values([TASK, { ...TASK, id: "t2", rank: 9 }]);

        // send the low-ranked task whole, and the high-ranked one without its title
        const page = await first(
            new Feed(database, TABLES),
            { tasks: { table: task, scopes: ["inbox"] } },
            AUDIENCE,
        );
        expect(
            page.changes.map((change) => [change.row.id, change.row.title, change.concealed]),
        ).toEqual([
            ["t1", "write", undefined],
            ["t2", undefined, ["title"]],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "follow rows whose visibility the audience changes on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [...TABLES, note]);
        await database.insert(task).values([TASK, { ...TASK, id: "t2" }]);

        // see the tasks a note names, and re-decide the tasks a note's change names
        const audience: Audience = {
            ...EVERYONE,
            watches: [{ table: note, scopes: ["grants"] }],
            admits: async (table, rows) => {
                const granted = new Set(
                    (await database.select({ id: note.id }).from(note)).map((row) => row.id),
                );

                return new Set(
                    rows.flatMap((row, index) =>
                        table !== task || granted.has(row.id as string) ? [index] : [],
                    ),
                );
            },
            where: (table) =>
                table === task ? sql`${task.id} IN (SELECT ${note.id} FROM ${note})` : sql`true`,
            key: "granted",
            dependents: async (change: Change) =>
                change.table === note ? [{ table: task, key: change.key }] : [],
        };
        const grant = {
            title: "grant",
            scope: "grants",
            summary: null,
            views: 0n,
            labels: [],
            editedAt: new Date(0),
        };
        const pages = feed.subscribe(
            { tasks: { table: task, scopes: ["inbox"] } },
            undefined,
            AbortSignal.timeout(5000),
            {
                audience,
            },
        );

        // hold nothing, then the granted task, then nothing once the grant goes
        const [snapshot] = await until(pages, (page) => page.complete);
        const granted = until(pages, (page) => page.changes.length > 0);
        await database.insert(note).values({ ...grant, id: "t1" });
        const [entered] = await granted;
        const revoked = until(pages, (page) => page.changes.length > 0);
        await database.delete(note).where(eq(note.id, "t1"));
        const [left] = await revoked;
        expect([
            snapshot!.changes,
            entered!.changes.map((change) => [change.operation, change.row.id]),
            left!.changes.map((change) => [change.operation, change.row.id]),
        ]).toEqual([[], [["insert", "t1"]], [["delete", "t1"]]]);
    },
);

test.for(TEST_DIALECTS)(
    "start over with a snapshot when access changes beyond a page on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [...TABLES, note]);
        await database.insert(task).values(TASK);
        const audience: Audience = {
            ...EVERYONE,
            watches: [{ table: note, scopes: ["grants"] }],
            dependents: async (change: Change) => (change.table === note ? "everything" : []),
        };
        const pages = feed.subscribe(
            { tasks: { table: task, scopes: ["inbox"] } },
            undefined,
            AbortSignal.timeout(5000),
            { audience },
        );

        // follow a snapshot, then another once access changes everything
        await until(pages, (page) => page.complete);
        const again = until(pages, (page) => page.reset);
        await database.insert(note).values({
            id: "n",
            title: "grant",
            scope: "grants",
            summary: null,
            views: 0n,
            labels: [],
            editedAt: new Date(0),
        });
        const [snapshot] = await again;
        expect([snapshot!.reset, snapshot!.changes.map((change) => change.row.id)]).toEqual([
            true,
            ["t1"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "refuse queries whose windows outgrow the feed's capacity on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, TABLES, { capacity: 3 });
        await database
            .insert(project)
            .values(["a", "b"].map((id) => ({ id, scope: "inbox", name: id })));
        await database.insert(task).values(
            [0, 1, 2, 3].map((index) => ({
                id: `t${index}`,
                scope: "inbox",
                projectId: index < 2 ? "a" : "b",
                state: "open",
                rank: index,
                points: null,
                title: null,
                isSecret: false,
            })),
        );

        // hold two tasks per project, four in all, beyond a capacity of three
        const pages = feed.subscribe(
            {
                projects: {
                    table: project,
                    scopes: ["inbox"],
                    include: {
                        tasks: {
                            table: task,
                            on: { kind: "key", column: "projectId", parent: "id" },
                            order: [{ column: "rank", direction: "asc" }],
                            limit: 2,
                        },
                    },
                },
            },
            undefined,
            AbortSignal.timeout(5000),
        );
        const refused = await take(pages, (page) => page.complete).then(
            () => undefined,
            (error: { code: string; message: string }) => [error.code, error.message],
        );
        expect(refused).toEqual(["OVER_CAPACITY", "queries hold 4 rows and groups, more than 3"]);
    },
);

test.for(TEST_DIALECTS)("refuse subscribers beyond a feed's limit on %s", async (dialect) => {
    const feed = new Feed(await open(dialect), TABLES, { subscribers: 1 });
    const query = { tasks: { table: task, scopes: ["inbox"] } };
    const signal = AbortSignal.timeout(5000);

    // serve the first subscriber, and refuse the second while it follows
    const served = feed.subscribe(query, undefined, signal);
    await served.next();
    const refused = await feed
        .subscribe(query, undefined, signal)
        .next()
        .then(
            () => undefined,
            (error: { code: string; message: string }) => [error.code, error.message],
        );
    await served.return(undefined);
    expect(refused).toEqual(["OVERLOADED", "feed serves 1 subscribers already"]);
});

test.for(TEST_DIALECTS)(
    "page catch-up changes at transaction boundaries on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, TABLES);
        await database.insert(project).values({ id: "p1", scope: "inbox", name: "a" });
        const start = await database.log.position();
        const signal = AbortSignal.timeout(5000);
        const tasks = (from: number, count: number) =>
            Array.from({ length: count }, (_, index) => ({ ...TASK, id: `t${from + index}` }));

        // commit one transaction of 1500 tasks, then two of 700
        for (const [from, count] of [
            [0, 1500],
            [1500, 700],
            [2200, 700],
        ] as const) {
            await database.transaction(async (transaction) => {
                await transaction.insert(task).values(tasks(from, count));
            });
        }

        // send the large transaction whole, and close a page once it holds a page and a transaction ends
        const pages: QueryPage[] = await take(
            feed.subscribe({ tasks: { table: task, scopes: ["inbox"] } }, start, signal),
            (page) => page.changes.some((change) => change.row.id === "t2899"),
        );
        expect(pages.map((page) => page.changes.length)).toEqual([1500, 1400]);
    },
);
