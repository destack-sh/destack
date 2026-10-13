import type { CallableName, CallOutput } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";

import { RequestId } from "@destack/runtime/request";
import { v7 } from "uuid";
import { ObjectStore } from "../src/server/index.ts";
import { objectDatabase, task, taskVersion } from "./schema.ts";
import { openSpace, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/runtime/test";
import { testEvents } from "../src/test/index.ts";
import { userContext } from "./fixture/user.ts";
import { request } from "./fixture/request.ts";

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
        const store = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: [task, taskVersion],
            database,
            callKey: testCallKey,
            events: await testEvents(database),
            origin: {
                package: task.package,
            },
        });
        const context = userContext("user-1", spaceId);
        const execute = <Name extends CallableName<typeof taskVersion>>(
            name: Name,
            input: Record<string, unknown>,
        ): Promise<CallOutput<typeof taskVersion, Name>> =>
            request(
                store,
                taskVersion,
                name,
                { spaceId, requestId: RequestId.create(Date.now()), ...input },
                context,
            );
        const first = await execute("create", { parentId: taskId, title: "Plan" });
        const second = await execute("create", { parentId: taskId, title: "Ship" });
        expect([first.number, second.number]).toEqual([1, 2]);

        // point the task at each version in turn through its own update
        const point = async (id: string, revision: number) =>
            store
                .mutate(context.caller(), {
                    id: RequestId.create(Date.now()),
                    calls: [task.call("update", { spaceId, id: taskId, revision, currentId: id })],
                })
                .then(([written]) => written);
        const pointed = await point(first.id, 1);
        expect((await point(second.id, pointed.revision)).currentId).toBe(second.id);
    },
);
