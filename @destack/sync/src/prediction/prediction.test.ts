import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { asc, eq, TABLE, type DatabaseConnection, type Table } from "@destack/db";
import { Replica, replicaTables } from "../replica/replica.ts";
import type { Page } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import { first, note, open, replicate, tag } from "../test/fixture.ts";
import type { Call, Mutation } from "../call/index.ts";
import { mutation, Prediction, predictionTables } from "./prediction.ts";
import { schema, found, zip } from "@destack/schema";

/** The release the calls were made against. */
const RELEASE = "2026.9.0";

/** The source log's epoch. */
const EPOCH = "01996ab0-0000-7000-8000-000000000001";

/** The request identifiers of three mutations, in order. */
const IDS = [
    "01996ab0-0000-7000-8000-00000000000a",
    "01996ab0-0000-7000-8000-00000000000b",
    "01996ab0-0000-7000-8000-00000000000c",
] as const;

/** The note identifiers the three mutations create, in order. */
const NOTES = ["a", "b", "c"] as const;

/** The input of a note's creation. */
const NoteInput = schema.object({ id: schema.string(), title: schema.string() });

/** Predict a mutation by creating each named note. */
async function predict(transaction: DatabaseConnection, predicted: Mutation): Promise<void> {
    for (const call of predicted.calls) {
        const { id, title } = NoteInput.parse(call.input);
        await transaction.insert(note).values({ ...first, id, title });
    }
}

/** Predict notes over some tables. */
function predicting(tables: readonly Table[], prediction = predict): Prediction {
    return new Prediction({ tables, predict: prediction, reads: () => [note[TABLE].sqlName] });
}

/** A complete page of a source's changes. */
function page(
    sequence: number,
    rows: readonly { readonly id: string; readonly title: string }[],
    options: { readonly reset?: boolean } = {},
): Page {
    return {
        reset: options.reset ?? false,
        complete: true,
        changes: rows.map((row) => ({
            table: note[TABLE].sqlName,
            operation: "insert" as const,
            row: note[TABLE].encode({ ...first, ...row }),
        })),
        position: { epoch: EPOCH, sequence },
    };
}

/** The failure a server rejects a mutation with. */
const FORBIDDEN = { code: "FORBIDDEN", status: 403, message: "forbidden" } as const;

/** Record a push's outcomes: the first mutation executed at a sequence, the second rejected. */
async function pushed(
    client: DatabaseConnection,
    prediction: Prediction,
    sequence: number,
): Promise<void> {
    await prediction.acknowledge(client, IDS[0], { epoch: EPOCH, sequence });
    await client.transaction(async (transaction) => {
        await prediction.reject(transaction, IDS[1], FORBIDDEN);
        await transaction.log.asReplica(() => prediction.revert(transaction));
        await prediction.replay(transaction);
    });
}

test("rebase predicted mutations onto source pages until the source executes or rejects them", async () => {
    const client = await open("sqlite", [note, ...replicaTables, ...predictionTables]);
    const prediction = predicting([note]);
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const titles = async () =>
        (await client.select().from(note).orderBy(asc(note.id))).map((row) => [row.id, row.title]);
    const outcomes = async () =>
        Promise.all(IDS.map(async (id) => found(await prediction.outcomes(client, [id]), id).kind));
    await replicate(notes, client, [page(1, [], { reset: true })], prediction);

    // predict three notes
    for (const [id, created] of zip(IDS, NOTES)) {
        const input = { id: created, title: "Local" };
        await prediction.add(client, id, "tab-1", async (transaction) => {
            const calls = [{ method: "note.create", release: RELEASE, input }];
            await predict(transaction, { id, calls });

            return { calls, result: undefined };
        });
    }
    expect(await titles()).toEqual([
        ["a", "Local"],
        ["b", "Local"],
        ["c", "Local"],
    ]);
    expect((await prediction.pending(client)).map((entry) => entry.id)).toEqual([...IDS]);

    // drop the rejected prediction and replace the executed one once a page reaches it
    await pushed(client, prediction, 2);
    await replicate(notes, client, [page(2, [{ id: "a", title: "Server" }])], prediction);
    expect(await titles()).toEqual([
        ["a", "Server"],
        ["c", "Local"],
    ]);
    expect(await outcomes()).toEqual(["executed", "rejected", "pending"]);
    expect((await prediction.outcomes(client, [IDS[1]])).get(IDS[1])).toEqual({
        kind: "rejected",
        error: FORBIDDEN,
    });
    expect((await prediction.pending(client)).map((entry) => entry.id)).toEqual([IDS[2]]);

    // acknowledge a mutation and drop it once a snapshot has it
    await prediction.acknowledge(client, IDS[2], { epoch: EPOCH, sequence: 3 });
    expect(await prediction.pending(client)).toEqual([]);
    await replicate(
        notes,
        client,
        [
            page(
                3,
                [
                    { id: "a", title: "Server" },
                    { id: "c", title: "Server" },
                ],
                { reset: true },
            ),
        ],
        prediction,
    );
    expect(await titles()).toEqual([
        ["a", "Server"],
        ["c", "Server"],
    ]);
    expect(await outcomes()).toEqual(["executed", "rejected", "executed"]);

    // forget the rejection once read
    await prediction.forget(client, "tab-1", IDS[1]);
    expect((await client.select({ id: mutation.id }).from(mutation)).length).toBe(0);
});

test("drop a group every predicted row left, without a count measuring it", async () => {
    // keep one group of notes by title
    const client = await open("sqlite", [note, ...replicaTables, ...predictionTables]);
    const prediction = predicting([note]);
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const titles: Omit<Query, "scopes"> = {
        table: note,
        aggregate: { groupBy: ["title"], values: { last: { function: "max", column: "id" } } },
    };
    await replicate(
        notes,
        client,
        [
            {
                ...page(1, [{ id: "a", title: "Server" }], { reset: true }),
                results: [
                    { query: "titles", group: { title: "Server" }, values: { last: "a" }, rows: 1 },
                ],
            },
        ],
        prediction,
    );

    // predict a retitle moving the group's only row
    await prediction.add(client, IDS[0], "tab-1", async (transaction) => {
        await transaction.update(note).set({ title: "Local" }).where(eq(note.id, "a"));

        return {
            calls: [{ method: "note.update", release: RELEASE, input: { id: "a" } }],
            result: undefined,
        };
    });
    expect(await notes.results(client, "titles", titles, prediction)).toEqual([
        { group: { title: "Local" }, values: { last: "a" } },
    ]);
});

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "refuse predicting on PostgreSQL, whose log sequences changes only at commit",
    async () => {
        const database = await open("postgresql", [note, ...replicaTables, ...predictionTables]);
        const prediction = predicting([note]);
        await expect(
            prediction.add(database, IDS[0], "tab-1", async () => ({
                calls: [],
                result: undefined,
            })),
        ).rejects.toThrow(
            "a prediction runs on SQLite, whose log sequences changes within a transaction",
        );
    },
);

test("rebase predictions only onto pages changing a table they read", async () => {
    const client = await open("sqlite", [note, tag, ...replicaTables, ...predictionTables]);
    let predictions = 0;
    const counted = async (transaction: DatabaseConnection, predicted: Mutation) => {
        predictions += 1;
        await predict(transaction, predicted);
    };
    const prediction = predicting([note, tag], counted);
    const copy = new Replica({ name: "notes", scope: "inbox", tables: [note, tag] });
    await replicate(copy, client, [page(1, [], { reset: true })], prediction);

    // predict a note
    await prediction.add(client, IDS[0], "tab-1", async (transaction) => {
        const calls = [
            { method: "note.create", release: RELEASE, input: { id: "a", title: "Local" } },
        ];
        await predict(transaction, { id: IDS[0], calls });

        return { calls, result: undefined };
    });

    // apply a page of tags, then a page of notes
    const tags: Page = {
        reset: false,
        complete: true,
        changes: [
            {
                table: tag[TABLE].sqlName,
                operation: "insert",
                row: tag[TABLE].encode({ id: "t", scope: "inbox", name: "urgent" }),
            },
        ],
        position: { epoch: EPOCH, sequence: 2 },
    };
    await replicate(copy, client, [tags], prediction);
    const afterTags = predictions;
    await replicate(copy, client, [page(3, [{ id: "b", title: "Server" }])], prediction);
    expect([afterTags, predictions]).toEqual([0, 1]);
});

test("read pending mutations up to a limit, and count each state with branch edits pending", async () => {
    // predict three notes and a fourth on a branch
    const client = await open("sqlite", [note, ...replicaTables, ...predictionTables]);
    const prediction = predicting([note]);
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    await replicate(notes, client, [page(1, [], { reset: true })], prediction);
    const add = (id: string, key: string) =>
        prediction.add(client, id, "tab-1", async (transaction) => {
            const calls = [
                { method: "note.create", release: RELEASE, input: { id: key, title: "Local" } },
            ];
            await predict(transaction, { id, calls });

            return { calls, result: undefined };
        });
    for (const [id, created] of zip(IDS, NOTES)) {
        await add(id, created);
    }
    await prediction.checkout(client, "draft");
    await add("01996ab0-0000-7000-8000-00000000000d", "d");
    await prediction.checkout(client, undefined);
    const before = await prediction.inspect(client);
    const limited = (await prediction.pending(client, { limit: 2 })).map((entry) => entry.id);

    // drop the first once its page has it, reject the second, and acknowledge the third
    await pushed(client, prediction, 2);
    await replicate(notes, client, [page(2, [{ id: "a", title: "Server" }])], prediction);
    await prediction.acknowledge(client, IDS[2], { epoch: EPOCH, sequence: 3 });

    // read the first pending ones up to the limit, and count the branch edit as pending
    expect({ limited, before, after: await prediction.inspect(client) }).toEqual({
        limited: [IDS[0], IDS[1]],
        before: { pending: 4, executed: 0, rejected: 0 },
        after: { pending: 1, executed: 1, rejected: 1 },
    });
});

test("predict a checked-out branch's rows under its edits, and push the edits to their branch", async () => {
    // keep a branch's rows in a list a page of tags announces
    const client = await open("sqlite", [note, tag, ...replicaTables, ...predictionTables]);
    const server: Call[] = [];
    const prediction = new Prediction({
        tables: [note, tag],
        predict,
        reads: () => [note[TABLE].sqlName],
        branches: {
            tables: [tag[TABLE].sqlName],
            isOpen: async () => true,
            apply: (transaction, branch) => predict(transaction, { id: branch, calls: server }),
        },
    });
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note, tag] });
    await replicate(notes, client, [page(1, [], { reset: true })], prediction);
    const call = (key: string): Call => ({
        method: "note.create",
        release: RELEASE,
        input: { id: key, title: key },
    });
    const add = (id: string, key: string, isMainLine?: boolean) =>
        prediction.add(
            client,
            id,
            "tab-1",
            async (transaction) => {
                await predict(transaction, { id, calls: [call(key)] });

                return { calls: [call(key)], result: undefined };
            },
            isMainLine === undefined ? {} : { isMainLine },
        );
    const titles = async () =>
        (await client.select({ title: note.title }).from(note).orderBy(asc(note.title))).map(
            (row) => row.title,
        );

    // write on the main line, then edit a checked-out branch
    await add(IDS[0], "m");
    await prediction.checkout(client, "branch-1");
    await add(IDS[1], "c");
    const edited = await titles();

    // show the branch's rows under the edit once a page announces them
    server.push(call("a"), call("x"));
    const announced: Page = {
        reset: false,
        complete: true,
        changes: [
            {
                table: tag[TABLE].sqlName,
                operation: "insert",
                row: tag[TABLE].encode({ id: "t", scope: "inbox", name: "branch" }),
            },
        ],
        position: { epoch: EPOCH, sequence: 2 },
    };
    await replicate(notes, client, [announced], prediction);
    const received = await titles();

    // write on the main line below the branch, then show the main line alone
    await add(IDS[2], "n", true);
    const below = await titles();
    await prediction.checkout(client, undefined);
    expect({
        edited,
        received,
        below,
        main: await titles(),
        pending: await prediction.pending(client),
    }).toEqual({
        edited: ["c", "m"],
        received: ["a", "c", "m", "x"],
        below: ["a", "c", "m", "n", "x"],
        main: ["m", "n"],
        pending: [
            { id: IDS[0], calls: [call("m")] },
            { id: IDS[1], calls: [call("c")], branch: "branch-1" },
            { id: IDS[2], calls: [call("n")] },
        ],
    });
});
