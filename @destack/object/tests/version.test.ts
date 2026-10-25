import { testSimulation } from "../src/test/index.ts";
import type { CallableName, MethodOutput } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { schema, Uuid } from "@destack/schema";

import { RequestId, Scope } from "@destack/sync";
import { objectDatabase, task, taskVersion } from "./schema.ts";
import { openSpace, spacePrincipal, space } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users } from "./fixture/user.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space containing the objects. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

test("number versions within their parent and point the parent at its current one", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(objectDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const taskId = schema.identifier("task").parse(`task-${Uuid.v7(world)}`);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(task.table).values({
                id: taskId,
                scope: spaceId,
                ownerId: users.frank,
                title: "Plan",
                createdAt: world.clock.now(),
                updatedAt: world.clock.now(),
            }),
    );

    // create versions numbered from one, taking the number the runtime assigns
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([task, taskVersion]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: {
            package: task.package,
        },
    });
    const context = userContext(users.frank, spaceId, { environment: world });
    const execute = <Name extends CallableName<typeof taskVersion>>(
        name: Name,
        input: NoInfer<RequestInput<typeof taskVersion, Name>>,
    ): Promise<MethodOutput<typeof taskVersion, Name>> =>
        request(
            store,
            taskVersion,
            name,
            spaceId,
            { requestId: RequestId.create(world), ...input },
            context,
        );
    const first = await execute("create", { parentId: taskId, title: "Plan" });
    const second = await execute("create", { parentId: taskId, title: "Ship" });
    expect([first.number, second.number]).toEqual([1, 2]);

    // point the task at each version in turn through its update
    const point = async (id: string, revision: number) =>
        store
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [task.call("update", { spaceId, id: taskId, revision, currentId: id })],
            })
            .then(([written]) => written);
    const pointed = await point(first.id, 1);
    expect((await point(second.id, pointed.revision)).currentId).toBe(second.id);
});
