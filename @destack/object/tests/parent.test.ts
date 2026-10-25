import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { accessRelationship, principal, Relationship } from "@destack/access";
import { eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema, Uuid } from "@destack/schema";
import { comment, folder, objectDatabase, task } from "./schema.ts";

/** The space containing the objects. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

test("store parents, delete children and every relationship with them, and nest trees under themselves", async () => {
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(objectDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const now = world.clock.now();
    const record = { scope: spaceId, createdAt: now, updatedAt: now };

    // comment on a task, relating a viewer to each
    const taskId = schema.identifier("task").parse(`task-${Uuid.v7(world)}`);
    const commentId = schema.identifier("comment").parse(`comment-${Uuid.v7(world)}`);
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(task.table)
                .values({ ...record, id: taskId, ownerId: users.owner, title: "Plan" }),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(comment.table)
                .values({ ...record, id: commentId, parentId: taskId, text: "Looks good" }),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(accessRelationship).values(
                [task.reference(spaceId, taskId), comment.reference(spaceId, commentId)].map(
                    (object) =>
                        Relationship.encode({
                            id: schema
                                .identifier("relationship")
                                .parse(`relationship-${Uuid.v7(world)}`),
                            object,
                            relation: "viewer",
                            subject: principal.user.reference("universe", users.bob),
                            createdAt: record.createdAt,
                            expiresAt: null,
                        }),
                ),
            ),
    );

    // delete the comments with the task, and the relationships of both
    await database.transaction(
        async (transaction) =>
            await transaction.delete(task.table).where(eq(task.table.id, taskId)),
    );
    expect([
        await database.select().from(comment.table),
        await database.select().from(accessRelationship),
    ]).toEqual([[], []]);

    // nest a folder under a root folder of the same type
    const root = schema.identifier("folder").parse(`folder-${Uuid.v7(world)}`);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(folder.table).values({ ...record, id: root, name: "Work" }),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(folder.table).values({
                ...record,
                id: schema.identifier("folder").parse(`folder-${Uuid.v7(world)}`),
                parentId: root,
                name: "Projects",
            }),
    );
    const nested = await database
        .select()
        .from(folder.table)
        .where(eq(folder.table.parentId, root));
    expect(nested.map((row) => row.name)).toEqual(["Projects"]);
    expect([comment.parent?.object, folder.parent?.object]).toEqual([task, folder]);
});
