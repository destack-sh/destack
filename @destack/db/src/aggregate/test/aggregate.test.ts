import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../../test/database.ts";
import { asc, eq } from "../../index.ts";
import { archive, item, list, lists, plainItem } from "./fixture.ts";

/** Open a migrated database holding lists and their items. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    const test = await TestDatabase.create(dialect, lists, { isMigrated: true });
    onTestFinished(() => test.close());

    return test.database;
}

test.for(TEST_DIALECTS)(
    "keep counts, sums and maximums of the rows referencing each row on %s",
    async (dialect) => {
        const database = await open(dialect);
        const read = async () =>
            (
                await database
                    .select({
                        id: list.id,
                        items: list.items,
                        done: list.done,
                        points: list.points,
                        largest: list.largest,
                    })
                    .from(list)
                    .orderBy(asc(list.id))
            ).map((row) => [row.id, row.items, row.done, row.points, row.largest]);

        // start every list at zero
        await database.insert(list).values([{ id: "a" }, { id: "b" }]);
        expect(await read()).toEqual([
            ["a", 0, 0, 0, null],
            ["b", 0, 0, 0, null],
        ]);

        // count and sum inserted items, and those matching a value
        await database.insert(item).values([
            { id: "1", listId: "a", points: 3, isDone: true },
            { id: "2", listId: "a", points: 5, isDone: false },
            { id: "3", listId: "b", points: 2, isDone: true },
        ]);
        expect(await read()).toEqual([
            ["a", 2, 1, 8, 5],
            ["b", 1, 1, 2, 2],
        ]);

        // move an item between lists, change its values, and delete one
        await database.update(item).set({ listId: "b", points: 7 }).where(eq(item.id, "2"));
        await database.update(item).set({ isDone: false }).where(eq(item.id, "3"));
        await database.delete(item).where(eq(item.id, "1"));
        expect(await read()).toEqual([
            ["a", 0, 0, 0, null],
            ["b", 2, 0, 9, 7],
        ]);

        // leave aggregates alone while copying rows a source derived
        await database.transaction(async (transaction) => {
            await transaction.log.copying(async () => {
                await transaction
                    .insert(item)
                    .values({ id: "4", listId: "a", points: 1, isDone: true });
            });
        });
        expect(await read()).toEqual([
            ["a", 0, 0, 0, null],
            ["b", 2, 0, 9, 7],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "compute declared aggregates afresh over the rows present on %s",
    async (dialect) => {
        // hold items before the lists keep any aggregate
        const test = await TestDatabase.create(dialect, [plainItem, list], { isMigrated: true });
        onTestFinished(() => test.close());
        await test.database.insert(list).values([{ id: "a" }]);
        await test.database.insert(plainItem).values([
            { id: "1", listId: "a", points: 3, isDone: true },
            { id: "2", listId: "a", points: 5, isDone: false },
        ]);

        // compute each aggregate once declared, then keep it current
        const database = await test.connect(lists);
        onTestFinished(() => database.close());
        await database.migrate(lists);
        const read = async () =>
            (await database.select().from(list)).map((row) => [
                row.items,
                row.done,
                row.points,
                row.largest,
            ]);
        expect(await read()).toEqual([[2, 1, 8, 5]]);
        await database.delete(item).where(eq(item.id, "2"));
        expect(await read()).toEqual([[1, 1, 3, 3]]);
    },
);

test.for(TEST_DIALECTS)(
    "keep an aggregate the holding table declares over another table's rows on %s",
    async (dialect) => {
        const test = await TestDatabase.create(dialect, [plainItem, archive], { isMigrated: true });
        onTestFinished(() => test.close());
        const { database } = test;
        const read = async () =>
            (await database.select().from(archive).orderBy(asc(archive.id))).map((row) => [
                row.id,
                row.done,
            ]);

        // count the done items of each archive's list as they change
        await database.insert(archive).values([{ id: "a" }, { id: "b" }]);
        await database.insert(plainItem).values([
            { id: "1", listId: "a", points: 3, isDone: true },
            { id: "2", listId: "a", points: 5, isDone: false },
            { id: "3", listId: "b", points: 2, isDone: true },
        ]);
        await database.update(plainItem).set({ isDone: true }).where(eq(plainItem.id, "2"));
        await database.delete(plainItem).where(eq(plainItem.id, "3"));
        expect(await read()).toEqual([
            ["a", 2],
            ["b", 0],
        ]);
    },
);

test("refuse an aggregate naming a table the database does not declare", async () => {
    // declare the archive without the items it counts
    await expect(TestDatabase.create("sqlite", [archive], { isMigrated: true })).rejects.toThrow(
        "aggregate of aggregate_item into aggregate_archive names undeclared table aggregate_item",
    );
});
