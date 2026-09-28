import { expect, onTestFinished, test } from "@destack/test";
import { asc, defineTable, eq, integer, text } from "../../index.ts";
import { connect } from "../bun/connection.ts";
import { relayHub } from "../../test/relay.ts";
import { relayNotifier } from "../../log/notifier.ts";
import { connectShared, Party, serveDatabase, type Message } from "./shared.ts";

/** Notes a party writes. */
const note = defineTable("shared_note", {
    /** The note's identifier. */
    id: text("id").primaryKey(),
    /** The note's position. */
    rank: integer("rank").notNull(),
});

test("run a party's statements and transactions on the owner's connection, notifying its commits", async () => {
    // serve the owner's database to a party
    const join = relayHub<Message>();
    const owner = await connect(":memory:", [note], { notifier: relayNotifier(join()) });
    onTestFinished(() => owner.close());
    await owner.migrate([note]);
    const stop = serveDatabase(owner.$client, join());
    onTestFinished(stop);
    const party = connectShared(join(), "tab-2", [note]);
    onTestFinished(() => party.close());

    // write and read through the owner
    await party.insert(note).values({ id: "a", rank: 1 });
    await owner.insert(note).values({ id: "b", rank: 2 });
    const ranks = async () =>
        (await party.select().from(note).orderBy(asc(note.id))).map((row) => [row.id, row.rank]);
    expect(await ranks()).toEqual([
        ["a", 1],
        ["b", 2],
    ]);

    // commit a transaction and roll back a failing one
    await party.transaction(async (transaction) => {
        await transaction.update(note).set({ rank: 3 }).where(eq(note.id, "a"));
        await transaction.insert(note).values({ id: "c", rank: 4 });
    });
    await expect(
        party.transaction(async (transaction) => {
            await transaction.delete(note).where(eq(note.id, "a"));
            throw new Error("undo");
        }),
    ).rejects.toThrow("undo");
    expect(await ranks()).toEqual([
        ["a", 3],
        ["b", 2],
        ["c", 4],
    ]);

    // wake the party on the owner's commit
    const waiting = party.log.until(
        async () => (await party.select().from(note)).length === 4,
        AbortSignal.timeout(2000),
    );
    await owner.insert(note).values({ id: "d", rank: 5 });
    expect(await waiting).toBe(true);
});

test("hold requests until an owner serves, and fail requests a replaced owner left unanswered", async () => {
    // ask before an owner serves
    const join = relayHub<Message>();
    const first = await connect(":memory:", [note]);
    onTestFinished(() => first.close());
    await first.migrate([note]);
    const party = new Party(join(), "tab-2");
    onTestFinished(() => party.close());
    const held = party.begin("deferred");
    const stopFirst = serveDatabase(first.$client, join());
    const transaction = await held;
    expect(transaction.id).toBe(0);

    // fail an unanswered request once a second owner serves
    stopFirst();
    const unanswered = party.request({
        type: "statement",
        method: "all",
        sql: "SELECT 1",
        parameters: [],
        isRaw: false,
        isSafe: false,
    });
    const second = await connect(":memory:", [note]);
    onTestFinished(() => second.close());
    onTestFinished(serveDatabase(second.$client, join()));
    await expect(unanswered).rejects.toMatchObject({ code: "OWNER_CHANGED" });

    // fail a step of the first owner's transaction
    const replaced = party.begin("deferred");
    const stale = party.request(
        {
            type: "statement",
            method: "all",
            sql: "SELECT 1",
            parameters: [],
            isRaw: false,
            isSafe: false,
            transaction: transaction.id,
        },
        transaction.owner,
    );
    await expect(stale).rejects.toMatchObject({ code: "OWNER_CHANGED" });
    expect((await replaced).id).toBe(0);
});
