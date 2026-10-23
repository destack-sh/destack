import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { accessRelationship, principal, Relationship } from "@destack/access";
import { eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { v7 } from "uuid";
import { comment, folder, objectDatabase, task } from "./schema.ts";

/** The space containing the objects. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

test("store parents, delete children and every relationship with them, and nest trees under themselves", async () => {
    const storage = await TestDatabase.create(objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const now = Date.now();
    const record = { scope: spaceId, createdAt: now, updatedAt: now };

    // comment on a task, relating a viewer to each
    const taskId = schema.identifier("task").parse(`task-${v7()}`);
    const commentId = schema.identifier("comment").parse(`comment-${v7()}`);
    await database
        .insert(task.table)
        .values({ ...record, id: taskId, ownerId: users.owner, title: "Plan" });
    await database
        .insert(comment.table)
        .values({ ...record, id: commentId, parentId: taskId, text: "Looks good" });
    await database.insert(accessRelationship).values(
        [task.reference(spaceId, taskId), comment.reference(spaceId, commentId)].map((object) =>
            Relationship.encode({
                id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                object,
                relation: "viewer",
                subject: principal.user.reference("universe", users.bob),
                createdAt: record.createdAt,
                expiresAt: null,
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
    const root = schema.identifier("folder").parse(`folder-${v7()}`);
    await database.insert(folder.table).values({ ...record, id: root, name: "Work" });
    await database.insert(folder.table).values({
        ...record,
        id: schema.identifier("folder").parse(`folder-${v7()}`),
        parentId: root,
        name: "Projects",
    });
    const nested = await database
        .select()
        .from(folder.table)
        .where(eq(folder.table.parentId, root));
    expect(nested.map((row) => row.name)).toEqual(["Projects"]);
    expect([comment.parent?.object, folder.parent?.object]).toEqual([task, folder]);
});
