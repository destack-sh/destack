import { testSimulation } from "../src/test/index.ts";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { users } from "./fixture/user.ts";
import { implementPackage } from "../src/server/index.ts";
import { expect, expectTypeOf, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { schema, type Identifier, Uuid } from "@destack/schema";
import { CallError } from "@destack/object";
import { project, task, tasksDatabase, tasksPackage } from "./fixture/task.ts";
import type { MethodInput } from "../src/index.ts";
import { principal } from "@destack/access";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents } from "../src/test/index.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space with the projects. */
const spaceId = schema.identifier("space").parse(`space-01996ab0-0000-7000-8000-00000000a00a`);

test("plan a project with its members, assignees, guarded budgets and comments", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(tasksDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);

    // serve the space's tasks to users identified by their bearer credential
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(tasksPackage, {})],
            chain: async () => undefined,
            environment: world,
            origin: { package: Package.of(tasksPackage) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(database, world),
            database,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());
    const as = (caller: Identifier<"user">) => served.as(caller).client(tasksPackage);
    const alice = as(users.alice);
    const bob = as(users.bob);
    const carol = as(users.carol);
    const dave = as(users.dave);

    // let the owner create a project and a budgeted task assigned to another user
    const launch = await alice.project.create({
        spaceId,
        requestId: RequestId.create(world),
        name: "Launch",
    });
    expect(launch.ownerId).toBe(users.alice);
    const announce = await alice.task.create({
        spaceId,
        requestId: RequestId.create(world),
        parentId: launch.id,
        title: "Write the announcement",
        assigneeId: users.bob,
        priority: "high",
        budget: 500,
        due: 1_790_000_000_000,
    });
    expect([announce.authorId, announce.status, announce.budget, announce.due]).toEqual([
        users.alice,
        "open",
        500,
        1_790_000_000_000,
    ]);

    // let the assignee read and work the task without planning it or seeing its budget
    const assigned = await bob.task.get({ spaceId, id: announce.id });
    expect([assigned.title, "budget" in assigned]).toEqual(["Write the announcement", false]);
    await bob.task.start({ spaceId, id: announce.id, requestId: RequestId.create(world) });
    expect(
        await refusal(
            bob.task.update({
                spaceId,
                id: announce.id,
                requestId: RequestId.create(world),
                revision: 2,
                title: "Rename",
            }),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: plan"]);
    expect(await refusal(bob.project.get({ spaceId, id: launch.id }))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `project ${launch.id} not found`,
    ]);

    // let a user ask to join, and plan once the owner accepts
    const asked = await carol.project.invite({
        spaceId,
        id: launch.id,
        requestId: RequestId.create(world),
        relationship: { relation: "member", subject: person(users.carol) },
        purpose: "help with the launch",
    });
    expect(
        (await alice.project.invitations({ spaceId, id: launch.id })).items.map(
            (invitation) => invitation.purpose,
        ),
    ).toEqual(["help with the launch"]);
    await alice.project.accept({
        spaceId,
        id: launch.id,
        requestId: RequestId.create(world),
        invitationId: asked.id,
    });
    const listed = await carol.task.list({
        spaceId,
        where: { parentId: launch.id },
    });
    expect(listed.items.map((item) => [item.title, item.status, "budget" in item])).toEqual([
        ["Write the announcement", "active", false],
    ]);
    await carol.task.create({
        spaceId,
        requestId: RequestId.create(world),
        parentId: launch.id,
        title: "Book the venue",
    });
    expect(
        await refusal(
            carol.task.update({
                spaceId,
                id: announce.id,
                requestId: RequestId.create(world),
                revision: 2,
                budget: 900,
            }),
        ),
    ).toEqual(["FORBIDDEN", "FIELD_NOT_WRITABLE", "field budget is not writable"]);

    // let everyone reading a task comment on it
    await bob.comment.create({
        spaceId,
        requestId: RequestId.create(world),
        parentId: announce.id,
        text: "Draft is ready",
    });
    expect(
        (await carol.comment.list({ spaceId, where: { parentId: announce.id } })).items.map(
            (item) => [item.authorId, item.text],
        ),
    ).toEqual([[users.bob, "Draft is ready"]]);
    await bob.task.complete({
        spaceId,
        id: announce.id,
        requestId: RequestId.create(world),
    });

    // follow up a task of another space by its address
    const origin = ObjectReference.parse({
        packageId: task.package.id,
        type: "task",
        scope: schema.identifier("space").parse(`space-${Uuid.v7(world)}`),
        id: schema.identifier("task").parse(`task-${Uuid.v7(world)}`),
    });
    const followUp = await alice.task.create({
        spaceId,
        requestId: RequestId.create(world),
        parentId: launch.id,
        title: "Reuse last year's checklist",
        ...ObjectReference.columns("origin", origin),
    });
    expect(ObjectReference.at("origin", followUp)).toEqual(origin);

    // refuse requests for another space than the caller authenticated for
    const elsewhere = schema.identifier("space").parse(`space-${Uuid.v7(world)}`);
    expect(await refusal(alice.project.get({ spaceId: elsewhere, id: launch.id }))).toEqual([
        "NOT_FOUND",
        "SCOPE_OUTSIDE_REQUEST",
        `scope ${elsewhere} is outside the request's scope`,
    ]);

    // hide the project from users with no relation to it
    expect((await dave.project.list({ spaceId })).items).toEqual([]);
    expect(await refusal(dave.task.get({ spaceId, id: announce.id }))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `task ${announce.id} not found`,
    ]);
});

test("serve a handled type in place of the package's type", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(tasksDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);

    // run the handled project's create in place of the plain one
    const frozen = project.implement({
        methods: {
            create: async () => {
                throw CallError.http("CONFLICT", { message: "projects are frozen" });
            },
        },
    });
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(tasksPackage, { objects: [frozen] })],
            chain: async () => undefined,
            environment: world,
            origin: { package: Package.of(tasksPackage) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            database: storage.database,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());
    const alice = served.as(users.alice).client(tasksPackage);
    await expect(
        alice.project.create({ spaceId, requestId: RequestId.create(world), name: "Launch" }),
    ).rejects.toEqual(CallError.http("CONFLICT", { message: "projects are frozen" }));
});

/** Reference a user principal by identifier. */
function person(id: Identifier<"user">) {
    return principal.user.reference("universe", id);
}

test("type each written field of a call's input as the value it accepts", () => {
    // read defaulted, optional and identifier fields as their values, never as unknown
    type Update = MethodInput<typeof task, "update">;
    type Create = MethodInput<typeof task, "create">;
    expectTypeOf<Update["priority"]>().toEqualTypeOf<
        "low" | "normal" | "high" | "urgent" | undefined
    >();
    expectTypeOf<Update["due"]>().toEqualTypeOf<number | null | undefined>();
    expectTypeOf<Update["id"]>().toEqualTypeOf<string>();
    expectTypeOf<Update["assigneeId"]>().toEqualTypeOf<string | null | undefined>();
    expectTypeOf<Create["parentId"]>().toEqualTypeOf<string>();
    expectTypeOf<Create["id"]>().toEqualTypeOf<string | undefined>();
});
