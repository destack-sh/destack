import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { asc, eq, TABLE, type DatabaseConnection, encodeRow } from "@destack/db";
import { Replica, replicaTables } from "../replica/replica.ts";
import type { QueryPage } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import { first, note, open, replicate, tag } from "../test/fixture.ts";
import { mutation, outboxTables, Outbox, type Mutation } from "./outbox.ts";

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
            row: encodeRow(note, { ...first, ...row }),
        })),
        position: { epoch: EPOCH, sequence },
        ...(options.outcomes === undefined ? {} : { outcomes: options.outcomes }),
    };
}

test("rebase predicted mutations onto source pages until the source executes or rejects them", async () => {
    const client = await open("sqlite", [note, ...replicaTables, ...outboxTables]);
    const outbox = new Outbox([note], predict, () => [note[TABLE].sqlName]);
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const titles = async () =>
        (await client.select().from(note).orderBy(asc(note.id))).map((row) => [row.id, row.title]);
    const outcomes = async () =>
        Promise.all(IDS.map(async (id) => (await outbox.outcomes(client, [id])).get(id)!.kind));
    await replicate(notes, client, [page(1, [], { reset: true })], outbox);

    // predict three notes
    for (const [index, id] of IDS.entries()) {
        const input = { id: ["a", "b", "c"][index]!, title: "Local" };
        await outbox.add(client, id, "tab-1", async (transaction) => {
            const calls = [{ method: "note.create", input }];
            await predict(transaction, { id, calls });

            return { calls, result: undefined };
        });
    }
    expect(await titles()).toEqual([
        ["a", "Local"],
        ["b", "Local"],
        ["c", "Local"],
    ]);
    expect((await outbox.pending(client)).map((entry) => entry.id)).toEqual([...IDS]);

    // replace the executed prediction and drop the rejected one
    await replicate(
        notes,
        client,
        [
            page(2, [{ id: "a", title: "Server" }], {
                outcomes: [{ id: IDS[0] }, { id: IDS[1], error: { code: "FORBIDDEN" } }],
            }),
        ],
        outbox,
    );
    expect(await titles()).toEqual([
        ["a", "Server"],
        ["c", "Local"],
    ]);
    expect(await outcomes()).toEqual(["executed", "rejected", "pending"]);
    expect((await outbox.outcomes(client, [IDS[1]])).get(IDS[1])).toEqual({
        kind: "rejected",
        error: { code: "FORBIDDEN" },
    });
    expect((await outbox.pending(client)).map((entry) => entry.id)).toEqual([IDS[2]]);

    // acknowledge a mutation and drop it once a snapshot holds it
    await outbox.acknowledge(client, IDS[2], { epoch: EPOCH, sequence: 3 });
    expect(await outbox.pending(client)).toEqual([]);
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
        outbox,
    );
    expect(await titles()).toEqual([
        ["a", "Server"],
        ["c", "Server"],
    ]);
    expect(await outcomes()).toEqual(["executed", "rejected", "executed"]);

    // forget the rejection once read
    await outbox.forget(client, "tab-1", IDS[1]);
    expect((await client.select({ id: mutation.id }).from(mutation)).length).toBe(0);
});

test("drop a group every predicted row left, without a count measuring it", async () => {
    // hold one group of notes by title
    const client = await open("sqlite", [note, ...replicaTables, ...outboxTables]);
    const outbox = new Outbox([note], predict, () => [note[TABLE].sqlName]);
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
        outbox,
    );

    // predict a retitle moving the group's only row
    await outbox.add(client, IDS[0], "tab-1", async (transaction) => {
        await transaction.update(note).set({ title: "Local" }).where(eq(note.id, "a"));

        return { calls: [{ method: "note.update", input: { id: "a" } }], result: undefined };
    });
    expect(await notes.results(client, "titles", titles, outbox)).toEqual([
        { group: { title: "Local" }, values: { last: "a" } },
    ]);
});

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "refuse predicting on PostgreSQL, whose log sequences changes only at commit",
    async () => {
        const database = await open("postgresql", [note, ...replicaTables, ...outboxTables]);
        const outbox = new Outbox([note], predict, () => [note[TABLE].sqlName]);
        await expect(
            outbox.add(database, IDS[0], "tab-1", async () => ({ calls: [], result: undefined })),
        ).rejects.toThrow(
            "an outbox predicts on SQLite, whose log sequences changes within a transaction",
        );
    },
);

test("rebase predictions only onto pages changing a table they reach", async () => {
    const client = await open("sqlite", [note, tag, ...replicaTables, ...outboxTables]);
    let predictions = 0;
    const counted = async (transaction: DatabaseConnection, predicted: Mutation) => {
        predictions += 1;
        await predict(transaction, predicted);
    };
    const outbox = new Outbox([note, tag], counted, () => [note[TABLE].sqlName]);
    const copy = new Replica({ name: "notes", scope: "inbox", tables: [note, tag] });
    await replicate(copy, client, [page(1, [], { reset: true })], outbox);

    // predict a note
    await outbox.add(client, IDS[0], "tab-1", async (transaction) => {
        const calls = [{ method: "note.create", input: { id: "a", title: "Local" } }];
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
                row: encodeRow(tag, { id: "t", scope: "inbox", name: "urgent" }),
            },
        ],
        position: { epoch: EPOCH, sequence: 2 },
    };
    await replicate(copy, client, [tags], outbox);
    const afterTags = predictions;
    await replicate(copy, client, [page(3, [{ id: "b", title: "Server" }])], outbox);
    expect([afterTags, predictions]).toEqual([0, 1]);
});
