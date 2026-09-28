import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { integer, text } from "../table/column.ts";
import { Condition } from "../query/condition.ts";
import { Order } from "../query/order.ts";
import { eq } from "../query/predicate.ts";
import type { LogPosition } from "./position.ts";

/** The writes before each snapshot. */
const WRITES = 120;

/** Ranked items in folders. */
const item = defineTable(
    "snapshot_item",
    {
        /** The item's key. */
        id: text("id").primaryKey(),
        /** The folder the item lives in. */
        scope: text("scope").notNull(),
        /** The rank. */
        rank: integer("rank").notNull(),
        /** A label, sometimes missing. */
        label: text("label"),
    },
    { log: {} },
);

/** Revisions whose changes survive compaction. */
const revision = defineTable(
    "snapshot_revision",
    {
        /** The revision's key. */
        id: text("id").primaryKey(),
        /** The folder the revision lives in. */
        scope: text("scope").notNull(),
        /** The revised title. */
        title: text("title").notNull(),
    },
    { log: { tier: "history" } },
);

/** Draw a seeded pseudo-random number. */
function random(seed: { value: number }): number {
    seed.value = (seed.value * 1_103_515_245 + 12_345) % 2_147_483_648;

    return seed.value / 2_147_483_648;
}

test.for(TEST_DIALECTS)(
    "read tables as they were at every earlier position on %s",
    async (dialect) => {
        const test = await TestDatabase.create(dialect, [item], { isMigrated: true });
        onTestFinished(() => test.close());
        const database = test.database;
        const seed = { value: 19 };
        const where = Condition.all(Condition.eq("scope", "inbox"), Condition.gte("rank", 2));
        const order = [{ column: "rank", direction: "desc" as const }];

        // write at random and capture the rows at each position
        const captured: { readonly position: LogPosition; readonly rows: unknown[] }[] = [];
        for (let index = 0; index < WRITES; index += 1) {
            const id = `i${Math.floor(random(seed) * 12)}`;
            const values = {
                id,
                scope: random(seed) < 0.8 ? "inbox" : "archive",
                rank: Math.floor(random(seed) * 6),
                label: random(seed) < 0.3 ? null : "x",
            };
            if (random(seed) < 0.2) {
                await database.delete(item).where(eq(item.id, id));
            } else {
                await database
                    .insert(item)
                    .values(values)
                    .onConflictDoUpdate({ target: item.id, set: values });
            }
            const rows = await database.select().from(item);
            captured.push({ position: await database.log.position(), rows });
        }

        // read each position's snapshot
        for (const { position, rows } of captured) {
            const snapshot = database.log.at(position);
            const matching = (rows as { id: string; scope: string; rank: number }[]).filter(
                (row) => row.scope === "inbox" && row.rank >= 2,
            );
            const sorted = [...matching].sort((left, right) =>
                Order.rows(Order.complete(order, item), left, right),
            );
            const byKey = (list: readonly unknown[]) =>
                [...(list as { id: string }[])].sort((left, right) =>
                    left.id < right.id ? -1 : 1,
                );
            expect([
                position.sequence,
                byKey(await snapshot.rows(item, where)),
                (await snapshot.row(item, { id: "i3" })) ?? null,
                await snapshot.ordered(item, { where, order, count: 3 }),
            ]).toEqual([
                position.sequence,
                byKey(matching),
                (rows as { id: string }[]).find((row) => row.id === "i3") ?? null,
                sorted.slice(0, 3),
            ]);
        }
    },
);

test.for(TEST_DIALECTS)(
    "read history past compaction, and refuse compacted windows and other epochs on %s",
    async (dialect) => {
        const test = await TestDatabase.create(dialect, [item, revision], { isMigrated: true });
        onTestFinished(() => test.close());
        const database = test.database;
        const everything = Condition.all();

        // write, take a position, change and compact
        await database.insert(item).values({ id: "a", scope: "inbox", rank: 1, label: null });
        await database.insert(revision).values({ id: "r", scope: "inbox", title: "First" });
        const position = await database.log.position();
        await database.update(revision).set({ title: "Second" }).where(eq(revision.id, "r"));
        await database.delete(item).where(eq(item.id, "a"));
        await database.log.compact(Date.now() + 1);
        const snapshot = database.log.at(position);

        // restore the history table and refuse the window table
        expect(await snapshot.rows(revision, everything)).toEqual([
            { id: "r", scope: "inbox", title: "First" },
        ]);
        await expect(snapshot.rows(item, everything)).rejects.toMatchObject({
            code: "CHANGES_COMPACTED",
        });

        // refuse reads after a new epoch
        await database.log.renew();
        await expect(snapshot.rows(revision, everything)).rejects.toMatchObject({
            code: "STALE_EPOCH",
        });
    },
);

test.for(TEST_DIALECTS)(
    "read the latest commit and the rows at it inside a writing transaction on %s",
    async (dialect) => {
        const test = await TestDatabase.create(dialect, [item], { isMigrated: true });
        onTestFinished(() => test.close());
        const database = test.database;
        await database.insert(item).values({ id: "i1", scope: "inbox", rank: 1, label: "x" });
        const committed = await database.log.position();

        // write inside a transaction and read the head and a snapshot
        const [position, rows, row] = await database.transaction(async (transaction) => {
            await transaction.update(item).set({ rank: 5 }).where(eq(item.id, "i1"));
            await transaction.insert(item).values({ id: "i2", scope: "inbox", rank: 2 });
            const position = await transaction.log.position();
            const snapshot = transaction.log.at(position);

            return [
                position,
                await snapshot.rows(item, Condition.eq("scope", "inbox")),
                await snapshot.row(item, { id: "i2" }),
            ] as const;
        });

        // show the latest commit without the transaction's writes
        expect([position, rows, row]).toEqual([
            committed,
            [{ id: "i1", scope: "inbox", rank: 1, label: "x" }],
            undefined,
        ]);
    },
);
