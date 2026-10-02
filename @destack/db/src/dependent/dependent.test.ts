import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { DatabaseError } from "../error/error.ts";
import { asc, eq } from "../index.ts";
import { photo, post, remark, section } from "./test/fixture.ts";

test.for(TEST_DIALECTS)(
    "delete the rows referencing a deleted row of their type, also below foreign key cascades, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [remark, post, section, photo], {
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
        await expect(database.delete(photo).where(eq(photo.id, "a"))).rejects.toThrow(
            new DatabaseError(
                "BROKEN_REFERENCE",
                "the change would leave a reference to a missing record",
            ),
        );
        expect(await database.select().from(photo)).toEqual([{ id: "a" }]);
        expect(await read()).toEqual(["2", "4"]);
    },
);
