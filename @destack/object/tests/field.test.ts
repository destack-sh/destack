import { expect, onTestFinished, test } from "@destack/test";
import { createInsertSchema, eq } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { describeObject } from "../src/inspect/index.ts";
import { objectDatabase, task } from "./schema.ts";

test.each(TEST_DIALECTS)(
    "hold qualified references to objects of another scope on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;

        // store a task referring to a task of another space by scope and identifier
        const origin = {
            scope: identifier("space").parse(`space-${v7()}`),
            id: identifier("task").parse(`task-${v7()}`),
        };
        const id = identifier("task").parse(`task-${v7()}`);
        await database.insert(task.table).values({
            id,
            scope: identifier("space").parse(`space-${v7()}`),
            owner: "user-1",
            title: "Follow up",
            origin,
            createdAt: Date.now(),
            updatedAt: Date.now(),
        });
        const [stored] = await database.select().from(task.table).where(eq(task.table.id, id));
        expect(stored!.origin).toEqual(origin);

        // reject a reference to an object of another type
        expect(
            createInsertSchema(task.table).shape.origin.safeParse({
                scope: origin.scope,
                id: `comment-${v7()}`,
            }).success,
        ).toBe(false);
        expect(describeObject(task).fields).toEqual({ estimate: { read: "plan", write: "plan" } });
    },
);
