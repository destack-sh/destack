import { expect, refusal, test } from "@destack/test";
import { nth, schema } from "@destack/schema";
import { eq, sql, type Table, Expression, type Change, type Row } from "@destack/db";
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
import { Clock } from "@destack/environment";

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

/** Decide whether the test audience lets its subscriber edit a task: one ranked below five. */
function isEditable(row: Row): boolean {
    return typeof row["rank"] === "number" && row["rank"] < 5;
}

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

    return nth(pages, 0);
}

test("refuse queries reading what the audience or the feed hides", async () => {
    const feed = new Feed(await open(), TABLES, { clock: Clock.system });
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

    // refuse reads of concealable columns, unserved tables and rows the audience decides in memory
    expect(
        await Promise.all([
            reason({ table: task, scopes: ["inbox"], where: { title: "write" } }, AUDIENCE),
            reason({ table: note, scopes: ["inbox"] }),
            reason(
                {
                    table: task,
                    scopes: ["inbox"],
                    extras: { label: Expression.column("title") },
                    orderBy: { label: "asc" },
                },
                AUDIENCE,
            ),
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
        "feed does not read note",
        "query query reads concealable columns of task: title",
        "relations cannot read rows of task: the audience decides them in memory",
    ]);
});

test("conceal the columns an audience hides and list them", async () => {
    const database = await open();
    await database.insert(task).values([TASK, { ...TASK, id: "t2", rank: 9 }]);

    // send the low-ranked task whole and the high-ranked one without its title
    const sent = await first(
        new Feed(database, TABLES, { clock: Clock.system }),
        { tasks: { table: task, scopes: ["inbox"] } },
        AUDIENCE,
    );
    expect(
        sent.changes.map((change) => [change.row["id"], change.row["title"], change.concealed]),
    ).toEqual([
        ["t1", "write", undefined],
        ["t2", undefined, ["title"]],
    ]);
});

test("follow rows whose visibility the audience changes", async () => {
    const database = await open();
    const feed = new Feed(database, [...TABLES, note], { clock: Clock.system });
    await database.insert(task).values([TASK, { ...TASK, id: "t2" }]);

    // see the tasks a note references
    const audience: Audience = {
        ...EVERYONE,
        watches: [{ table: note, scopes: ["grants"] }],
        visible: async (table, rows) => {
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
        decides: async (change: Change) => change.table === note,
    };
    const grant = {
        title: "grant",
        scope: "grants",
        subtitle: null,
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
    const snapshot = nth(await until(pages, (sent) => sent.complete), 0);
    const granted = until(pages, (sent) => sent.changes.length > 0);
    await database.insert(note).values({ ...grant, id: "t1" });
    const entered = nth(await granted, 0);
    const revoked = until(pages, (sent) => sent.changes.length > 0);
    await database.delete(note).where(eq(note.id, "t1"));
    const left = nth(await revoked, 0);
    // send each access change as the difference to what the subscriber has, never a reset
    expect([
        snapshot.changes,
        [entered.reset, entered.changes.map((change) => [change.operation, change.row["id"]])],
        [left.reset, left.changes.map((change) => [change.operation, change.row["id"]])],
    ]).toEqual([[], [false, [["insert", "t1"]]], [false, [["delete", "t1"]]]]);
});

test("send the rows each query selects under the query's permission, and a row once its permission comes", async () => {
    const database = await open();
    const feed = new Feed(database, TABLES, { clock: Clock.system });
    await database
        .insert(task)
        .values([
            TASK,
            { ...TASK, id: "t2", rank: 9 },
            { ...TASK, id: "t3", rank: 9, state: "done" },
        ]);

    // let the subscriber edit the tasks ranked below five, and list every task
    const audience: Audience = {
        ...EVERYONE,
        watches: [{ table: task, scopes: ["inbox"] }],
        where: (table, permission) =>
            table === task && permission === "edit" ? sql`${task.rank} < 5` : sql`true`,
        visible: async (table, rows, _position, permission) =>
            new Set(
                rows.flatMap((row, index) =>
                    table !== task || permission !== "edit" || isEditable(row) ? [index] : [],
                ),
            ),
        key: "editor",
    };

    // select the open tasks the subscriber edits and every done task
    const pages = feed.subscribe(
        {
            open: {
                table: task,
                scopes: ["inbox"],
                where: { state: "open" },
                permission: "edit",
            },
            done: { table: task, scopes: ["inbox"], where: { state: "done" } },
        },
        undefined,
        AbortSignal.timeout(5000),
        { audience },
    );
    const snapshot = nth(await until(pages, (sent) => sent.complete), 0);
    const ranked = until(pages, (sent) => sent.changes.length > 0);
    await database.update(task).set({ rank: 2 }).where(eq(task.id, "t2"));
    const entered = nth(await ranked, 0);

    // send the editable open task and the done one, then the open task once it becomes editable
    expect([
        snapshot.changes
            .map((change) => schema.string().parse(change.row["id"]))
            .toSorted((left, right) => left.localeCompare(right)),
        entered.changes.map((change) => [change.operation, change.row["id"]]),
    ]).toEqual([["t1", "t3"], [["insert", "t2"]]]);
});

test("refuse queries whose windows outgrow the feed's capacity", async () => {
    const database = await open();
    const feed = new Feed(database, TABLES, { clock: Clock.system, capacity: 3 });
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
    expect(refused).toEqual(["OVER_CAPACITY", "queries have 4 rows and groups, more than 3"]);
});

test("refuse subscribers beyond a feed's limit", async () => {
    const feed = new Feed(await open(), TABLES, { clock: Clock.system, subscribers: 1 });
    const query = { tasks: { table: task, scopes: ["inbox"] } };
    const signal = AbortSignal.timeout(5000);

    // refuse a second subscriber
    const served = feed.subscribe(query, undefined, signal);
    await served.next();
    const refused = await refusal(feed.subscribe(query, undefined, signal).next());
    await served.return(undefined);
    expect(refused).toEqual(["OVERLOADED", "feed serves 1 subscribers already"]);
});

test("page catch-up changes between transactions", async () => {
    const database = await open();
    const feed = new Feed(database, TABLES, { clock: Clock.system, pageSize: 4 });
    await database.insert(project).values({ id: "p1", scope: "inbox", name: "a" });
    const start = await database.log.position();
    const signal = AbortSignal.timeout(5000);
    const tasks = (from: number, count: number) =>
        Array.from({ length: count }, (_, index) => ({ ...TASK, id: `t${from + index}` }));

    // commit one transaction of more tasks than a page and two of fewer
    for (const [from, count] of [
        [0, 6],
        [6, 3],
        [9, 3],
    ] as const) {
        await database.transaction(async (transaction) => {
            await transaction.insert(task).values(tasks(from, count));
        });
    }

    // send the large transaction whole and split pages at transaction ends
    const pages: Page[] = await take(
        feed.subscribe({ tasks: { table: task, scopes: ["inbox"] } }, start, signal),
        (sent) => sent.changes.some((change) => change.row["id"] === "t11"),
    );
    expect(pages.map((sent) => sent.changes.length)).toEqual([6, 6]);
});
