import { expect, onTestFinished, test } from "@destack/test";
import { accessRelationship, principal } from "@destack/access";
import { eq } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { comment, folder, objectDatabase, task } from "./schema.ts";

/** The space containing the objects. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

test.each(TEST_DIALECTS)(
    "hold parents, delete children and every relationship with them, and nest trees under themselves on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        const now = Date.now();
        const record = { scope: spaceId, createdAt: now, updatedAt: now };

        // comment on a task, relating a viewer to each
        const taskId = identifier("task").parse(`task-${v7()}`);
        const commentId = identifier("comment").parse(`comment-${v7()}`);
        await database
            .insert(task.table)
            .values({ ...record, id: taskId, owner: "alice", title: "Plan" });
        await database
            .insert(comment.table)
            .values({ ...record, id: commentId, parentId: taskId, text: "Looks good" });
        await database.insert(accessRelationship).values(
            [task.reference(spaceId, taskId), comment.reference(spaceId, commentId)].map(
                (object) => ({
                    ...record,
                    id: identifier("relationship").parse(`relationship-${v7()}`),
                    objectScope: object.scope,
                    packageId: object.packageId,
                    type: object.type,
                    objectId: object.id,
                    relation: "viewer",
                    subjectPackageId: principal.user.definition.packageId,
                    subjectType: principal.user.name,
                    subjectScope: "global",
                    subjectId: "bob",
                }),
            ),
        );

        // delete the comments with the task, and the relationships of both
        await database.delete(task.table).where(eq(task.table.id, taskId));
        expect([
            await database.select().from(comment.table),
            await database.select().from(accessRelationship),
        ]).toEqual([[], []]);

        // nest a folder under a root folder of the same type
        const root = identifier("folder").parse(`folder-${v7()}`);
        await database.insert(folder.table).values({ ...record, id: root, name: "Work" });
        await database.insert(folder.table).values({
            ...record,
            id: identifier("folder").parse(`folder-${v7()}`),
            parentId: root,
            name: "Projects",
        });
        const nested = await database
            .select()
            .from(folder.table)
            .where(eq(folder.table.parentId, root));
        expect(nested.map((row) => row.name)).toEqual(["Projects"]);
        expect([comment.parent?.object, folder.parent?.object]).toEqual([task, folder]);
    },
);
