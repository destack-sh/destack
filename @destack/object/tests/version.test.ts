import type { CallableName, CallOutput } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { objectDatabase, task, taskVersion } from "./schema.ts";
import { openSpace, space } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";
import { userContext } from "./fixture/user.ts";

/** The space containing the objects. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

test.each(TEST_DIALECTS)(
    "number versions within their parent and point the parent at its current one on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId);
        const taskId = schema.identifier("task").parse(`task-${v7()}`);
        await database.insert(task.table).values({
            id: taskId,
            scope: spaceId,
            ownerId: "user-1",
            title: "Plan",
            createdAt: Date.now(),
            updatedAt: Date.now(),
        });

        // create versions numbered from one, taking the number the runtime assigns
        const server = new ObjectServer({
            objects: { task, taskVersion },
            database,
            callKey: testCallKey,
            origin: {
                package: task.package,
                service: "test",
            },
        });
        const context = userContext("user-1", spaceId);
        const execute = <Name extends CallableName<typeof taskVersion>>(
            name: Name,
            input: Record<string, unknown>,
        ): Promise<CallOutput<typeof taskVersion, Name>> =>
            server.call(
                taskVersion,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            );
        const first = await execute("create", { parentId: taskId, title: "Plan" });
        const second = await execute("create", { parentId: taskId, title: "Ship" });
        expect([first.number, second.number]).toEqual([1, 2]);

        // point the task at each version in turn through its own update
        const point = async (id: string, revision: number) =>
            server.call(
                task,
                "update",
                { spaceId, requestId: RequestId.create(), id: taskId, revision, currentId: id },
                context,
            );
        const pointed = await point(first.id, 1);
        expect((await point(second.id, pointed.revision)).currentId).toBe(second.id);

        // refuse updates to immutable versions
        expect(() =>
            defineObject({
                name: "draft",
                plural: "drafts",
                scope: space,
                nested: { in: task, receive: "write" },
                versioned: true,
                fields: { title: field.string() },
                permissions: ["write"],
                methods: (method) => ({
                    update: method.update("write"),
                }),
            }),
        ).toThrow(new TypeError("versions of draft are immutable and cannot be updated"));
    },
);
