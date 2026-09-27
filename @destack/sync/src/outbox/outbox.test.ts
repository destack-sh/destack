import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { asc, eq, TABLE, type DatabaseConnection, encodeRow } from "@destack/db";
import { Replica, REPLICA_TABLES } from "../replica/replica.ts";
import type { QueryPage } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import { first, note, open, replicate } from "../test/fixture.ts";
import { mutation, Outbox, type Mutation } from "./outbox.ts";

/** The source log's epoch every page belongs to. */
const EPOCH = "01996ab0-0000-7000-8000-000000000001";

/** The request identifiers of three mutations, in the order they were made. */
const IDS = [
    "01996ab0-0000-7000-8000-00000000000a",
    "01996ab0-0000-7000-8000-00000000000b",
    "01996ab0-0000-7000-8000-00000000000c",
] as const;

/** Predict a mutation's calls: each creates the note it names with its title. */
async function predict(transaction: DatabaseConnection, predicted: Mutation): Promise<void> {
    for (const call of predicted.calls) {
        const { id, title } = call.input as { id: string; title: string };
        await transaction.insert(note).values({ ...first, id, title });
    }
}

/** A complete page of a source's changes at a sequence. */
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
    const client = await open("sqlite", [note, ...REPLICA_TABLES, mutation]);
    const outbox = new Outbox([note], predict);
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const titles = async () =>
        (await client.select().from(note).orderBy(asc(note.id))).map((row) => [row.id, row.title]);
    const outcomes = async () =>
        Promise.all(IDS.map(async (id) => (await outbox.outcome(client, id)).kind));
    await replicate(notes, client, [page(1, [], { reset: true })], outbox);

    // predict three notes locally, each waiting for the source
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

    // replace the executed prediction with the source's row, and drop the rejected one
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
    expect(await outbox.outcome(client, IDS[1])).toEqual({
        kind: "rejected",
        error: { code: "FORBIDDEN" },
    });
    expect((await outbox.pending(client)).map((entry) => entry.id)).toEqual([IDS[2]]);

    // stop pushing an acknowledged mutation, and drop it once a snapshot holds its position
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

    // forget the rejection once its origin read it
    await outbox.forget(client, "tab-1", IDS[1]);
    expect((await client.select({ id: mutation.id }).from(mutation)).length).toBe(0);
});

test("drop a group every predicted row left, without a count measuring it", async () => {
    // hold one group of the notes by title, measured by the greatest identity only
    const client = await open("sqlite", [note, ...REPLICA_TABLES, mutation]);
    const outbox = new Outbox([note], predict);
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

    // predict a retitle that moves the group's only row into another group
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
        const database = await open("postgresql", [note, ...REPLICA_TABLES, mutation]);
        const outbox = new Outbox([note], predict);
        await expect(
            outbox.add(database, IDS[0], "tab-1", async () => ({ calls: [], result: undefined })),
        ).rejects.toThrow(
            "an outbox predicts on SQLite, whose log sequences changes within a transaction",
        );
    },
);
