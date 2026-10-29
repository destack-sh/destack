import { expect, onTestFinished, test, refusal } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { asc, eq } from "../index.ts";
import { photo, post, remark, section } from "./test/fixture.ts";
import { testSimulation } from "@destack/test/world";

test("delete the rows referencing a deleted row of their type, also below foreign key cascades", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([remark, post, section, photo], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const read = async () =>
        (await database.select({ id: remark.id }).from(remark).orderBy(asc(remark.id))).map(
            (row) => row.id,
        );

    // remark on two posts, a section and a photo
    await database.insert(post).values([{ id: "a" }, { id: "b" }]);
    await database.insert(section).values([{ id: "s", postId: "a" }]);
    await database.insert(photo).values([{ id: "a" }]);
    await database.insert(remark).values([
        { id: "1", subjectType: "post", subjectId: "a" },
        { id: "2", subjectType: "post", subjectId: "b" },
        { id: "3", subjectType: "section", subjectId: "s" },
        { id: "4", subjectType: "photo", subjectId: "a" },
    ]);

    // delete the first post's and its section's remarks
    await database.delete(post).where(eq(post.id, "a"));
    expect(await read()).toEqual(["2", "4"]);

    // refuse deleting the remarked photo
    expect(await refusal(() => database.delete(photo).where(eq(photo.id, "a")))).toEqual([
        "CONFLICT",
        "BROKEN_REFERENCE",
        "the change would leave a reference to a missing record",
    ]);
    expect(await database.select().from(photo)).toEqual([{ id: "a" }]);
    expect(await read()).toEqual(["2", "4"]);
});

test("copy a replica's rows of a table with dependents, marking it a replica's only before a delete", async () => {
    // open posts with remarks
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create([remark, post, section, photo], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.transaction(async (transaction) => {
        await transaction.insert(post).values([{ id: "a" }]);
        await transaction.insert(remark).values([{ id: "1", subjectType: "post", subjectId: "a" }]);
    });

    // insert as a replica without the marker, then delete beside the marker's insert and delete
    const counted = async (run: () => Promise<unknown>) => {
        const before = database.state.statements;
        await run();

        return database.state.statements - before;
    };
    const inserted = await counted(() =>
        database.transaction((transaction) =>
            transaction.log.asReplica(async () => {
                await transaction.insert(post).values([{ id: "b" }]);
            }),
        ),
    );
    const deleted = await counted(() =>
        database.transaction((transaction) =>
            transaction.log.asReplica(async () => {
                await transaction.delete(post).where(eq(post.id, "a"));
            }),
        ),
    );

    // keep the deleted post's remark, which its source deletes and sends apart
    const remarks = await database.select({ id: remark.id }).from(remark);
    expect({ inserted, deleted, remarks: remarks.map((row) => row.id) }).toEqual({
        inserted: 2,
        deleted: 4,
        remarks: ["1"],
    });
});
