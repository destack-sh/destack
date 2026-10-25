import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../../test/database.ts";
import { asc, eq } from "../../index.ts";
import { archive, item, list, lists, plainItem } from "./fixture.ts";
import { testSimulation } from "@destack/test/world";

/** Open a migrated database with lists and their items. */
async function open() {
    const storage = await TestDatabase.create(lists, {
        environment: testSimulation().environment("test"),
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    return storage.database;
}

test("maintain counts, sums and maximums of the rows referencing each row", async () => {
    const database = await open();
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

    // count and sum items
    await database.insert(item).values([
        { id: "1", listId: "a", points: 3, isDone: true },
        { id: "2", listId: "a", points: 5, isDone: false },
        { id: "3", listId: "b", points: 2, isDone: true },
    ]);
    expect(await read()).toEqual([
        ["a", 2, 1, 8, 5],
        ["b", 1, 1, 2, 2],
    ]);

    // move, change and delete items
    await database.update(item).set({ listId: "b", points: 7 }).where(eq(item.id, "2"));
    await database.update(item).set({ isDone: false }).where(eq(item.id, "3"));
    await database.delete(item).where(eq(item.id, "1"));
    expect(await read()).toEqual([
        ["a", 0, 0, 0, null],
        ["b", 2, 0, 9, 7],
    ]);

    // leave aggregates alone while copying derived rows
    await database.transaction(async (transaction) => {
        await transaction.log.asReplica(async () => {
            await transaction
                .insert(item)
                .values({ id: "4", listId: "a", points: 1, isDone: true });
        });
    });
    expect(await read()).toEqual([
        ["a", 0, 0, 0, null],
        ["b", 2, 0, 9, 7],
    ]);
});

test("compute declared aggregates afresh over the rows present", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // insert items before any aggregate
    const storage = await TestDatabase.create([plainItem, list], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await storage.database.insert(list).values([{ id: "a" }]);
    await storage.database.insert(plainItem).values([
        { id: "1", listId: "a", points: 3, isDone: true },
        { id: "2", listId: "a", points: 5, isDone: false },
    ]);

    // compute each declared aggregate, then maintain it
    const database = await storage.connect(lists);
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
});

test("maintain an aggregate the target table declares over another table's rows", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([plainItem, archive], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const read = async () =>
        (await database.select().from(archive).orderBy(asc(archive.id))).map((row) => [
            row.id,
            row.done,
        ]);

    // count each archive's done items
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
});
