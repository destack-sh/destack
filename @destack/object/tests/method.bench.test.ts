import { expect, test } from "@destack/test";
import { nth } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { users } from "./fixture/user.ts";
import { TestDatabase } from "@destack/db/test";

/** The calls of each verb counted, after as many warm-up calls. */
const CALLS = 200;

/** The rows a listed page holds. */
const PAGE_ROWS = 20;

/** The most statements one call of each verb runs on SQLite, its woken background work included. */
const STATEMENTS = {
    create: 10,
    get: 2,
    list: 0,
    update: 11,
    delete: 13,
    createNested: 12,
};

/** The statements PostgreSQL adds to a write, assigning its transaction identity. */
const POSTGRES_WRITE_STATEMENTS = 1;

test("run each object verb within its statement budget, reporting its statements per call", async ({
    annotate,
}) => {
    const { connect, environment, server, database } = await serveNotes();
    const alice = connect(users.alice);
    const request = () => ({ spaceId, requestId: RequestId.create(environment) });
    const statements = async () => {
        // count once the work the calls woke settles
        await server.idle();

        return database.state.statements;
    };

    // count a verb's statements per call, after warming it up
    const measure = async (
        verb: keyof typeof STATEMENTS,
        call: (index: number) => Promise<unknown>,
    ) => {
        for (let index = 0; index < CALLS; index++) {
            await call(index);
        }
        const counted = await statements();
        for (let index = CALLS; index < 2 * CALLS; index++) {
            await call(index);
        }
        const count = ((await statements()) - counted) / CALLS;
        await annotate(`${verb} statements per call ${count.toFixed(2)}`, "cost");

        return { verb, count };
    };

    // create the notebooks every other verb reads, changes and deletes
    const created: string[] = [];
    const counted = [
        await measure("create", async (index) => {
            created.push((await alice.notebook.create({ ...request(), name: `book ${index}` })).id);
        }),
        await measure("get", (index) => alice.notebook.get({ spaceId, id: nth(created, index) })),
        await measure("list", () => alice.notebook.list({ spaceId, limit: PAGE_ROWS })),
        await measure("update", (index) =>
            alice.notebook.update({
                ...request(),
                id: nth(created, index),
                name: `renamed ${index}`,
            }),
        ),
        await measure("createNested", (index) =>
            alice.note.create({
                ...request(),
                parentId: nth(created, index),
                title: `note ${index}`,
            }),
        ),
        await measure("delete", (index) =>
            alice.notebook.delete({ ...request(), id: nth(created, index) }),
        ),
    ];

    // stay within each verb's statement budget, PostgreSQL's writes assigning their transaction identity
    const extra = TestDatabase.target() === "postgresql" ? POSTGRES_WRITE_STATEMENTS : 0;
    const over = counted.filter(({ verb, count }) => {
        const budget = STATEMENTS[verb] + (verb === "get" || verb === "list" ? 0 : extra);

        return Math.round(count) > budget;
    });
    expect(over).toEqual([]);
});
