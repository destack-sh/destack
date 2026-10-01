import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { asc, eq, TABLE, type DatabaseConnection } from "@destack/db";
import { Replica, replicaTables } from "../replica/replica.ts";
import type { QueryPage } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import { first, note, open, replicate, tag } from "../test/fixture.ts";
import { mutation } from "../outbox/outbox.ts";
import type { Call, Mutation } from "../call/index.ts";
import { Prediction, predictionTables } from "./prediction.ts";

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

/** Predict a mutation by creating each named note. */
async function predict(transaction: DatabaseConnection, predicted: Mutation): Promise<void> {
    for (const call of predicted.calls) {
        const { id, title } = call.input as { id: string; title: string };
        await transaction.insert(note).values({ ...first, id, title });
    }
}

/** A complete page of a source's changes. */
function page(
    sequence: number,
    rows: readonly { readonly id: string; readonly title: string }[],
    options: Pick<QueryPage, "outcomes"> & { readonly reset?: boolean } = {},
): QueryPage {
    return {
        reset: options.reset ?? false,
        complete: true,
        changes: rows.map((row) => ({
            table: note[TABLE].sqlName,
            operation: "insert" as const,
            row: note.encode({ ...first, ...row }),
        })),
        position: { epoch: EPOCH, sequence },
        ...(options.outcomes === undefined ? {} : { outcomes: options.outcomes }),
    };
}

test("rebase predicted mutations onto source pages until the source executes or rejects them", async () => {
    const client = await open("sqlite", [note, ...replicaTables, ...predictionTables]);
    const prediction = new Prediction([note], predict, () => [note[TABLE].sqlName]);
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const titles = async () =>
        (await client.select().from(note).orderBy(asc(note.id))).map((row) => [row.id, row.title]);
    const outcomes = async () =>
        Promise.all(
            IDS.map(async (id) => (await prediction.outbox.outcomes(client, [id])).get(id)!.kind),
        );
    await replicate(notes, client, [page(1, [], { reset: true })], prediction);

    // predict three notes
    for (const [index, id] of IDS.entries()) {
        const input = { id: ["a", "b", "c"][index]!, title: "Local" };
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
    expect((await prediction.outbox.pending(client)).map((entry) => entry.id)).toEqual([...IDS]);

    // replace the executed prediction and drop the rejected one
    await replicate(
        notes,
        client,
        [
            page(2, [{ id: "a", title: "Server" }], {
                outcomes: [
                    { id: IDS[0] },
                    { id: IDS[1], error: { code: "FORBIDDEN", status: 403, message: "forbidden" } },
                ],
            }),
        ],
        prediction,
    );
    expect(await titles()).toEqual([
        ["a", "Server"],
        ["c", "Local"],
    ]);
    expect(await outcomes()).toEqual(["executed", "rejected", "pending"]);
    expect((await prediction.outbox.outcomes(client, [IDS[1]])).get(IDS[1])).toEqual({
        kind: "rejected",
        error: { code: "FORBIDDEN", status: 403, message: "forbidden" },
    });
    expect((await prediction.outbox.pending(client)).map((entry) => entry.id)).toEqual([IDS[2]]);

    // acknowledge a mutation and drop it once a snapshot holds it
    await prediction.outbox.acknowledge(client, IDS[2], { epoch: EPOCH, sequence: 3 });
    expect(await prediction.outbox.pending(client)).toEqual([]);
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
    await prediction.outbox.forget(client, "tab-1", IDS[1]);
    expect((await client.select({ id: mutation.id }).from(mutation)).length).toBe(0);
});

test("drop a group every predicted row left, without a count measuring it", async () => {
    // hold one group of notes by title
    const client = await open("sqlite", [note, ...replicaTables, ...predictionTables]);
    const prediction = new Prediction([note], predict, () => [note[TABLE].sqlName]);
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
        const prediction = new Prediction([note], predict, () => [note[TABLE].sqlName]);
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

test("rebase predictions only onto pages changing a table they reach", async () => {
    const client = await open("sqlite", [note, tag, ...replicaTables, ...predictionTables]);
    let predictions = 0;
    const counted = async (transaction: DatabaseConnection, predicted: Mutation) => {
        predictions += 1;
        await predict(transaction, predicted);
    };
    const prediction = new Prediction([note, tag], counted, () => [note[TABLE].sqlName]);
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
    const tags: QueryPage = {
        reset: false,
        complete: true,
        changes: [
            {
                table: tag[TABLE].sqlName,
                operation: "insert",
                row: tag.encode({ id: "t", scope: "inbox", name: "urgent" }),
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
    const prediction = new Prediction([note], predict, () => [note[TABLE].sqlName]);
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
    for (const [index, id] of IDS.entries()) {
        await add(id, ["a", "b", "c"][index]!);
    }
    await prediction.checkout(client, "draft");
    await add("01996ab0-0000-7000-8000-00000000000d", "d");
    await prediction.checkout(client, undefined);
    const before = await prediction.outbox.inspect(client);
    const limited = (await prediction.outbox.pending(client, { limit: 2 })).map(
        (entry) => entry.id,
    );

    // drop the first once its page holds it, reject the second, and acknowledge the third
    await replicate(
        notes,
        client,
        [
            page(2, [{ id: "a", title: "Server" }], {
                outcomes: [
                    { id: IDS[0] },
                    { id: IDS[1], error: { code: "FORBIDDEN", status: 403, message: "forbidden" } },
                ],
            }),
        ],
        prediction,
    );
    await prediction.outbox.acknowledge(client, IDS[2], { epoch: EPOCH, sequence: 3 });

    // read the first pending ones up to the limit, and count the branch edit as pending
    expect({ limited, before, after: await prediction.outbox.inspect(client) }).toEqual({
        limited: [IDS[0], IDS[1]],
        before: { pending: 4, executed: 0, rejected: 0 },
        after: { pending: 1, executed: 1, rejected: 1 },
    });
});

test("predict a checked-out branch's rows under its edits, and push the edits to their branch", async () => {
    // keep a branch's rows in a list a page of tags announces
    const client = await open("sqlite", [note, tag, ...replicaTables, ...predictionTables]);
    const server: Call[] = [];
    const prediction = new Prediction([note, tag], predict, () => [note[TABLE].sqlName], {
        tables: [tag[TABLE].sqlName],
        isOpen: async () => true,
        apply: (transaction, branch) => predict(transaction, { id: branch, calls: server }),
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
    const announced: QueryPage = {
        reset: false,
        complete: true,
        changes: [
            {
                table: tag[TABLE].sqlName,
                operation: "insert",
                row: tag.encode({ id: "t", scope: "inbox", name: "branch" }),
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
        pending: await prediction.outbox.pending(client),
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
