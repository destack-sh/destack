import { expect, refusal, test } from "@destack/test";
import { aligned } from "@destack/schema";
import { TEST_DIALECTS } from "@destack/db/test";
import { eq, sql, type Table, Expression, type Change, Relations } from "@destack/db";
import { Feed } from "./feed.ts";
import { EVERYONE, type Audience } from "./audience.ts";
import type { Page } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import {
    comment,
    note,
    open,
    page,
    project,
    relations,
    tag,
    task,
    taskTag,
    take,
    until,
} from "../test/fixture.ts";
import { AUDIENCE, MEMORY_AUDIENCE } from "./test/query.ts";

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
    const pages = await take(
        feed.subscribe(queries, undefined, AbortSignal.timeout(5000), { audience }),
        (entry) => entry.complete,
    );

    return aligned(pages, 0);
}

test.for(TEST_DIALECTS)(
    "refuse queries a subscription cannot keep exactly on %s",
    async (dialect) => {
        const feed = new Feed(await open(dialect), TABLES);
        const reason = async (query: Query, audience: Audience = EVERYONE) => {
            const pages = feed.subscribe({ query }, undefined, AbortSignal.timeout(5000), {
                audience,
            });
            const error = await pages.next().then(
                () => undefined,
                (thrown: unknown) => (thrown instanceof Error ? thrown.message : thrown),
            );

            return error;
        };

        // refuse invalid reads with their reasons
        expect(
            await Promise.all([
                reason({ table: task, scopes: ["inbox"], where: { title: "write" } }, AUDIENCE),
                reason({
                    table: task,
                    scopes: ["inbox"],
                    orderBy: { missing: "asc" },
                }),
                reason({ table: task, scopes: ["inbox"], limit: 0 }),
                reason({
                    table: task,
                    scopes: ["inbox"],
                    aggregate: { values: { total: { function: "sum", column: "state" } } },
                }),
                reason({ table: note, scopes: ["inbox"], where: { labels: "draft" } }),
                reason({ table: note, scopes: ["inbox"] }),
                reason({
                    table: task,
                    scopes: ["inbox"],
                    limit: 2,
                    aggregate: { values: { tasks: { function: "count" } } },
                }),
                reason(
                    {
                        table: task,
                        scopes: ["inbox"],
                        extras: { label: Expression.column("title") },
                        orderBy: { label: "asc" },
                    },
                    AUDIENCE,
                ),
                reason({
                    table: task,
                    scopes: ["inbox"],
                    extras: { rank: Expression.literal(1) },
                }),
                reason({
                    table: task,
                    scopes: ["inbox"],
                    extras: { label: Expression.column("title") },
                    aggregate: { values: { total: { function: "avg", column: "label" } } },
                }),
                reason({
                    table: project,
                    scopes: ["inbox"],
                    relations,
                    extras: { spent: Expression.rollup("sum", "tasks", "title") },
                }),
                reason({
                    table: task,
                    scopes: ["inbox"],
                    relations: new Relations(
                        new Map([
                            [
                                task,
                                {
                                    tasks: {
                                        table: task,
                                        cardinality: "many",
                                        on: {
                                            kind: "key",
                                            column: "projectId",
                                            parent: "projectId",
                                        },
                                    },
                                },
                            ],
                        ]),
                    ),
                    extras: { peers: Expression.rollup("count", "tasks") },
                }),
                reason({
                    table: task,
                    scopes: ["inbox"],
                    extras: { peers: Expression.rollup("count", "tasks") },
                }),
                reason(
                    {
                        table: project,
                        scopes: ["inbox"],
                        relations,
                        extras: { open: Expression.rollup("count", "tasks") },
                    },
                    MEMORY_AUDIENCE,
                ),
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
            "task has no relation tasks",
            "relations read no rows of task, which the audience decides in memory",
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "conceal the columns an audience hides and name them on %s",
    async (dialect) => {
        const database = await open(dialect);
        await database.insert(task).values([TASK, { ...TASK, id: "t2", rank: 9 }]);

        // send the low-ranked task whole and the high-ranked one without its title
        const sent = await first(
            new Feed(database, TABLES),
            { tasks: { table: task, scopes: ["inbox"] } },
            AUDIENCE,
        );
        expect(
            sent.changes.map((change) => [change.row["id"], change.row["title"], change.concealed]),
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

        // see the tasks a note names
        const audience: Audience = {
            ...EVERYONE,
            watches: [{ table: note, scopes: ["grants"] }],
            admits: async (table, rows) => {
                const granted = new Set(
                    (await database.select({ id: note.id }).from(note)).map((row) => row.id),
                );

                return new Set(
                    rows.flatMap((row, index) => {
                        const id = row["id"];

                        return table !== task || (typeof id === "string" && granted.has(id))
                            ? [index]
                            : [];
                    }),
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
            editedAt: 0,
        };
        const pages = feed.subscribe(
            { tasks: { table: task, scopes: ["inbox"] } },
            undefined,
            AbortSignal.timeout(5000),
            {
                audience,
            },
        );

        // select the granted task only while the grant lasts
        const snapshot = aligned(await until(pages, (sent) => sent.complete), 0);
        const granted = until(pages, (sent) => sent.changes.length > 0);
        await database.insert(note).values({ ...grant, id: "t1" });
        const entered = aligned(await granted, 0);
        const revoked = until(pages, (sent) => sent.changes.length > 0);
        await database.delete(note).where(eq(note.id, "t1"));
        const left = aligned(await revoked, 0);
        expect([
            snapshot.changes,
            entered.changes.map((change) => [change.operation, change.row["id"]]),
            left.changes.map((change) => [change.operation, change.row["id"]]),
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

        // start over after access changes everything
        await until(pages, (sent) => sent.complete);
        const again = until(pages, (sent) => sent.reset);
        await database.insert(note).values({
            id: "n",
            title: "grant",
            scope: "grants",
            summary: null,
            views: 0n,
            labels: [],
            editedAt: 0,
        });
        const snapshot = aligned(await again, 0);
        expect([snapshot.reset, snapshot.changes.map((change) => change.row["id"])]).toEqual([
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

        // exceed a capacity of three
        const pages = feed.subscribe(
            {
                projects: {
                    table: project,
                    scopes: ["inbox"],
                    relations,
                    with: {
                        tasks: {
                            orderBy: { rank: "asc" },
                            limit: 2,
                        },
                    },
                },
            },
            undefined,
            AbortSignal.timeout(5000),
        );
        const refused = await refusal(take(pages, (sent) => sent.complete));
        expect(refused).toEqual(["OVER_CAPACITY", "queries hold 4 rows and groups, more than 3"]);
    },
);

test.for(TEST_DIALECTS)("refuse subscribers beyond a feed's limit on %s", async (dialect) => {
    const feed = new Feed(await open(dialect), TABLES, { subscribers: 1 });
    const query = { tasks: { table: task, scopes: ["inbox"] } };
    const signal = AbortSignal.timeout(5000);

    // refuse a second subscriber
    const served = feed.subscribe(query, undefined, signal);
    await served.next();
    const refused = await refusal(feed.subscribe(query, undefined, signal).next());
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

        // commit one transaction of 1500 tasks and two of 700
        for (const [from, count] of [
            [0, 1500],
            [1500, 700],
            [2200, 700],
        ] as const) {
            await database.transaction(async (transaction) => {
                await transaction.insert(task).values(tasks(from, count));
            });
        }

        // send the large transaction whole and split pages at transaction ends
        const pages: Page[] = await take(
            feed.subscribe({ tasks: { table: task, scopes: ["inbox"] } }, start, signal),
            (sent) => sent.changes.some((change) => change.row["id"] === "t2899"),
        );
        expect(pages.map((sent) => sent.changes.length)).toEqual([1500, 1400]);
    },
);
