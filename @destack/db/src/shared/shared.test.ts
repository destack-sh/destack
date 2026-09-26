import { expect, onTestFinished, test } from "@destack/test";
import { migrate } from "../migration/database.ts";
import { asc, defineTable, eq, integer, text } from "../index.ts";
import { connect } from "../sqlite/turso/connection.ts";
import { channelHub } from "../test/channel.ts";
import { channelNotifier } from "../log/notifier.ts";
import { connectShared, Party, serveDatabase, type Message } from "./shared.ts";

/** Notes a party writes through the owner. */
const note = defineTable("shared_note", {
    /** The note's identifier. */
    id: text("id").primaryKey(),
    /** The note's position. */
    rank: integer("rank").notNull(),
});

test("run a party's statements and transactions on the owner's connection, notifying its commits", async () => {
    // serve the owner's database to a party
    const join = channelHub<Message>();
    const owner = await connect(":memory:", [note], { notifier: channelNotifier(join()) });
    onTestFinished(() => owner.close());
    await migrate(owner, [note]);
    const stop = serveDatabase(owner.$client, join());
    onTestFinished(stop);
    const party = connectShared(join(), "tab-2", [note]);
    onTestFinished(() => party.close());

    // write and read through the owner, and see the owner's own writes
    await party.insert(note).values({ id: "a", rank: 1 });
    await owner.insert(note).values({ id: "b", rank: 2 });
    const ranks = async () =>
        (await party.select().from(note).orderBy(asc(note.id))).map((row) => [row.id, row.rank]);
    expect(await ranks()).toEqual([
        ["a", 1],
        ["b", 2],
    ]);

    // commit a transaction as a whole, and roll back one that fails
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

    // wake the party's readers when the owner commits
    const waiting = party.log.until(
        async () => (await party.select().from(note)).length === 4,
        AbortSignal.timeout(2000),
    );
    await owner.insert(note).values({ id: "d", rank: 5 });
    expect(await waiting).toBe(true);
});

test("hold requests until an owner serves, and fail requests a replaced owner left unanswered", async () => {
    // ask before any owner serves, and answer once one does
    const join = channelHub<Message>();
    const first = await connect(":memory:", [note]);
    onTestFinished(() => first.close());
    await migrate(first, [note]);
    const party = new Party(join(), "tab-2");
    onTestFinished(() => party.close());
    const held = party.request({ type: "begin", mode: "deferred" });
    const stopFirst = serveDatabase(first.$client, join());
    expect(await held).toBe(0);

    // fail a request the first owner received but never answered, once a second owner serves
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
});
