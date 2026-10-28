import type { Environment } from "@destack/environment";
import { testSimulation } from "../src/test/index.ts";
import type { CallableName, MethodOutput } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { Page } from "@destack/sync";
import { Identifier, schema, Uuid } from "@destack/schema";

import { RequestId, Scope } from "@destack/sync";
import { comment, folder, objectDatabase, task, taskVersion, team } from "./schema.ts";
import { principal } from "@destack/access";
import { openSpace, spacePrincipal, space } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users } from "./fixture/user.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** How long a short grant lasts, in milliseconds. */
const GRANT_MILLISECONDS = 300;

/** The space containing the tasks. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** Serve two tasks of the first user and follow what the second user sees. */
async function serveTasks(world: Environment) {
    const storage = await TestDatabase.create(objectDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);

    // insert two tasks of the first user
    const insert = async (title: string) => {
        const id = Identifier.create("task", world);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(task.table).values({
                    id,
                    scope: spaceId,
                    ownerId: users.frank,
                    title,
                    estimate: 5,
                    createdAt: world.clock.now(),
                    updatedAt: world.clock.now(),
                }),
        );

        return id;
    };
    const first = await insert("First");
    const second = await insert("Second");

    // serve methods as each user and follow the tasks the second user sees
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([task, team, comment, taskVersion, folder]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: {
            package: task.package,
        },
    });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const context = userContext(users.grace, spaceId, {
        environment: world,
        signal: controller.signal,
    });
    const execute = async <
        Object extends typeof task | typeof team,
        Name extends CallableName<Object>,
    >(
        caller: Identifier<"user">,
        object: Object,
        name: Name,
        input: NoInfer<RequestInput<Object, Name>>,
    ): Promise<MethodOutput<Object, Name>> => {
        return request(
            store,
            object,
            name,
            spaceId,
            input,
            userContext(caller, spaceId, { environment: world }),
        );
    };
    const pages = store.source.relayed(
        {
            ...store.source.queriesPublication.subscription({
                name: "objects",
                scope: spaceId,
                target: spaceId,
                parameters: {},
            }),
            revision: { log: "follower", epoch: "follower", sequence: 1 },
        },
        context,
    );
    const next = async (): Promise<[boolean, string, unknown, unknown][]> => {
        // skip the bare pages that only move the position and read the next page of changes
        let page = await nextPage(pages);
        while (page.changes.length === 0) {
            page = await nextPage(pages);
        }

        return page.changes.map((change) => [
            page.reset,
            change.operation,
            change.row["title"],
            change.row["estimate"],
        ]);
    };

    // read a snapshot of every object type while none exist yet
    const snapshot: Page[] = [];
    while (snapshot.at(-1)?.complete !== true) {
        snapshot.push(await nextPage(pages));
    }
    expect(snapshot.flatMap((page) => page.changes)).toEqual([]);

    return { database, execute, next, first, second };
}

test("follow the tasks a caller sees as grants, changes and ownership move them", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { database, execute, next, first, second } = await serveTasks(world);

    // select the task a grant relates the caller to
    const viewer = principal.user.reference("universe", users.grace);
    const shared = await execute(users.frank, task, "grant", {
        id: first,
        requestId: RequestId.create(world),
        relation: "viewer",
        subject: viewer,
    });
    expect(await next()).toEqual([[false, "insert", "First", undefined]]);

    // send changes of visible tasks, never of invisible ones
    await execute(users.frank, task, "update", {
        id: second,
        requestId: RequestId.create(world),
        revision: 1,
        title: "Hidden",
    });
    await execute(users.frank, task, "update", {
        id: first,
        requestId: RequestId.create(world),
        revision: 1,
        title: "Renamed",
    });
    expect(await next()).toEqual([[false, "update", "Renamed", undefined]]);

    // select the task once ownership moves to the caller, and drop it when ownership leaves
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(task.table)
                .set({ ownerId: users.grace })
                .where(eq(task.table.id, second)),
    );
    expect(await next()).toEqual([[false, "insert", "Hidden", 5]]);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(task.table)
                .set({ ownerId: users.frank })
                .where(eq(task.table.id, second)),
    );
    expect(await next()).toEqual([[false, "delete", undefined, undefined]]);

    // leave the task once the grant is revoked
    await execute(users.frank, task, "revoke", {
        id: first,
        requestId: RequestId.create(world),
        relationshipId: shared.id,
    });
    expect(await next()).toEqual([[false, "delete", undefined, undefined]]);
});

test("follow the tasks a team grants once the caller joins the team", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { database, execute, next, first } = await serveTasks(world);

    // let the first user's team view the first task, before the caller belongs to it
    const teamId = schema.identifier("team").parse(`team-${Uuid.v7(world)}`);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(team.table).values({
                id: teamId,
                scope: spaceId,
                ownerId: users.frank,
                createdAt: world.clock.now(),
                updatedAt: world.clock.now(),
            }),
    );
    await execute(users.frank, task, "grant", {
        id: first,
        requestId: RequestId.create(world),
        relation: "viewer",
        subject: { ...team.reference(spaceId, teamId), relation: "member" },
    });

    // select the task once the caller joins the team, only a relationship on the team
    await execute(users.frank, team, "grant", {
        id: teamId,
        requestId: RequestId.create(world),
        relation: "member",
        subject: principal.user.reference("universe", users.grace),
    });
    expect(await next()).toEqual([[false, "insert", "First", undefined]]);
});

test("let go of a task once the grant relating it expires, with no change in the log", async () => {
    // run in a world of its own, moving its time while the stream waits
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { execute, next, first } = await serveTasks(world);

    // select a task through a grant that expires shortly
    await execute(users.frank, task, "grant", {
        id: first,
        requestId: RequestId.create(world),
        relation: "viewer",
        subject: principal.user.reference("universe", users.grace),
        expiresAt: world.clock.now() + GRANT_MILLISECONDS,
    });
    expect(await next()).toEqual([[false, "insert", "First", undefined]]);

    // drop it once the grant expires
    expect(await simulation.drive(next())).toEqual([[false, "delete", undefined, undefined]]);
});

/** Read the next page of a sync stream, refusing an ended stream. */
async function nextPage(pages: AsyncGenerator<Page>): Promise<Page> {
    const read = await pages.next();
    if (read.done === true) {
        throw new TypeError("the sync stream ended");
    }

    return read.value;
}
