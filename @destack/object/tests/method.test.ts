import type { CallableName } from "../src/index.ts";
import { Subject } from "@destack/sync";
import { reconciliation, testCallKey } from "@destack/service/test";
import { expect, expectTypeOf, onTestFinished, test } from "@destack/test";
import { ServiceError } from "@destack/service/error";
import { type Dialect, eq } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { aligned, present, schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { v7 } from "uuid";
import { ObjectServer, Settlement } from "../src/server/index.ts";
import {
    Call,
    defineObject,
    method,
    recoverable,
    settlement,
    suspendable,
    type ObjectProcedures,
} from "../src/index.ts";
import { describeObject } from "../src/inspect/index.ts";
import { comment, folder, objectDatabase, task, taskCopy, taskVersion } from "./schema.ts";
import { principal } from "@destack/access";
import { openSpace, space } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext } from "./fixture/user.ts";
import { any } from "./fixture/match.ts";

/** The space containing the tasks. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The routes derived procedures keep, by method name. */
const PROCEDURES = schema.record(
    schema.string(),
    schema.looseObject({
        "~orpc": schema.looseObject({
            route: schema.looseObject({ method: schema.string(), path: schema.string() }),
        }),
    }),
);

/** An object type named like the replica procedures every object type shares. */
const replica = defineObject({
    name: "replica",
    plural: "replicas",
    scope: space,
    fields: {},
    permissions: ["read"],
});

test("derive typed routes and describe each method with its permission, input and audit action", () => {
    // type each derived input and output from the method and the table
    type Procedures = ObjectProcedures<typeof task>;
    type UpdateInput = schema.Input<Procedures["update"]["~orpc"]["inputSchema"] & schema.Schema>;
    type GetOutput = schema.Infer<Procedures["get"]["~orpc"]["outputSchema"] & schema.Schema>;
    const id = schema.identifier("task").parse("task-01996ab0-0000-7000-8000-000000000004");
    expectTypeOf({
        spaceId,
        id,
        requestId: RequestId.create(),
        revision: 1,
        title: "Typed",
    }).toExtend<UpdateInput>();

    // require a request identifier in updates
    expectTypeOf({ spaceId, id, title: "x" }).not.toExtend<UpdateInput>();
    expectTypeOf("typed").toExtend<GetOutput["title"]>();

    // leave system methods out of the names and procedures callers see
    const machine = defineObject({
        name: "machine",
        plural: "machines",
        scope: space,
        fields: {},
        permissions: ["read"],
        methods: (builder) => ({
            get: builder.get("read"),
            reset: builder.mutation({ permission: null, isSystem: true }),
        }),
    });
    expectTypeOf<CallableName<typeof machine>>().toEqualTypeOf<"get">();
    expectTypeOf<keyof ObjectProcedures<typeof machine>>().toEqualTypeOf<"get">();

    // derive one route per method under the object's scope
    expect(
        Object.fromEntries(
            Object.entries(PROCEDURES.parse(task.procedures)).map(([name, procedure]) => [
                name,
                `${procedure["~orpc"].route.method} ${procedure["~orpc"].route.path}`,
            ]),
        ),
    ).toEqual({
        get: "GET /spaces/{spaceId}/tasks/{id}",
        list: "POST /spaces/{spaceId}/tasks/query",
        create: "POST /spaces/{spaceId}/tasks",
        update: "PATCH /spaces/{spaceId}/tasks/{id}",
        delete: "DELETE /spaces/{spaceId}/tasks/{id}",
        archive: "POST /spaces/{spaceId}/tasks/{id}/archive",
        export: "POST /spaces/{spaceId}/tasks/{id}/export",
        complete: "POST /spaces/{spaceId}/tasks/{id}/complete",
        reopen: "POST /spaces/{spaceId}/tasks/{id}/reopen",
        restore: "POST /spaces/{spaceId}/tasks/{id}/restore",
        purge: "POST /spaces/{spaceId}/tasks/{id}/purge",
        relationships: "GET /spaces/{spaceId}/tasks/{id}/relationships",
        grant: "POST /spaces/{spaceId}/tasks/{id}/relationships",
        revoke: "DELETE /spaces/{spaceId}/tasks/{id}/relationships/{relationshipId}",
        proposals: "GET /spaces/{spaceId}/tasks/{id}/proposals",
        propose: "POST /spaces/{spaceId}/tasks/{id}/proposals",
        accept: "POST /spaces/{spaceId}/tasks/{id}/proposals/{proposalId}/accept",
        decline: "DELETE /spaces/{spaceId}/tasks/{id}/proposals/{proposalId}",
        explain: "GET /spaces/{spaceId}/tasks/{id}/access",
    });

    // describe each method statically with its route, permission, input and audit action
    const description = describeObject(task);
    expect(description.methods["archive"]).toEqual({
        kind: "custom",
        permission: "write",
        mutates: true,
        isPredicted: false,
        route: { method: "POST", path: "/spaces/{spaceId}/tasks/{id}/archive" },
        input: any(Object),
        output: { kind: "value", schema: any(Object) },
        audit: {
            name: "task.archive",
            package: {
                id: task.policy.definition.packageId,
                name: "@destack/object",
                version: any(String),
            },
            targets: any(Object),
            details: any(Object),
        },
    });

    // take every writable field in an update, requiring only the target and request
    const updateInput = schema
        .looseObject({
            properties: schema.record(schema.string(), schema.unknown()),
            required: schema.array(schema.string()),
        })
        .parse(present(description.methods["update"], "the update method").input);
    expect([Object.keys(updateInput.properties), updateInput.required]).toEqual([
        ["spaceId", "id", "requestId", "revision", "title", "estimate", "currentId"],
        ["spaceId", "id", "requestId"],
    ]);
    expect(present(description.methods["get"], "the get method").audit).toBeUndefined();
});

test.each(TEST_DIALECTS)(
    "route every served object, create once per request identifier and take chosen identifiers on %s",
    async (dialect) => {
        const { server, database, execute, events, observed } = await serveTasks(dialect);

        // route the replica procedures and each served object
        expect(Object.keys(server.router())).toEqual([
            "replica",
            "task",
            "comment",
            "taskVersion",
            "folder",
        ]);
        expect(
            () =>
                new ObjectServer({
                    objects: { replica },
                    database,
                    callKey: testCallKey,
                    origin: {
                        package: task.package,
                        service: "test",
                    },
                }),
        ).toThrow(new TypeError("object replica takes the name of the shared replica procedures"));

        // create once per request identifier, replaying the original result
        const creation = { requestId: RequestId.create(), title: "Write the plan" };
        const created = await execute("create", creation);
        expect(observed().watermarks.at(-1)).toEqual({
            scope: spaceId,
            epoch: await database.log.epoch(),
            sequence: any(Number),
        });
        expect(await execute("create", creation)).toEqual(created);

        // take a chosen identifier and refuse a taken one
        const chosen = schema.identifier("task").parse(`task-${v7()}`);
        const drafted = { requestId: RequestId.create(), id: chosen, title: "Draft" };
        expect((await execute("create", drafted)).id).toBe(chosen);
        await expect(
            execute("create", { requestId: RequestId.create(), id: chosen, title: "Again" }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "task identifier is taken" });
        const ahead = schema
            .identifier("task")
            .parse(`task-${v7({ msecs: Date.now() + 3_600_000 })}`);
        const later = { requestId: RequestId.create(), id: ahead, title: "Later" };
        expect((await execute("create", later)).id).toBe(ahead);
        await database.delete(task.table).where(eq(task.table.id, chosen));
        await database.delete(task.table).where(eq(task.table.id, ahead));
        expect((await execute("list", {})).items.map((item) => item.title)).toEqual([
            "Write the plan",
        ]);

        // audit each creation once
        expect(await events()).toEqual([
            "task.create",
            "task.create",
            "task.create CONFLICT",
            "task.create",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "hide an object from strangers, update it at the observed revision, and keep sensitive fields out of rows on %s",
    async (dialect) => {
        const { database, execute, events, act } = await serveTasks(dialect);
        const created = await execute("create", {
            requestId: RequestId.create(),
            title: "Write the plan",
        });

        // hide the task from another user, and refuse them changes without revealing it
        act("user-2");
        expect((await execute("list", {})).items).toEqual([]);
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${created.id}`,
        });
        await expect(
            execute("update", {
                id: created.id,
                requestId: RequestId.create(),
                revision: created.revision,
                title: "Taken",
            }),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no task ${created.id}` });
        act("user-1");

        // update at the observed revision and reject a stale one
        const updated = await execute("update", {
            id: created.id,
            requestId: RequestId.create(),
            revision: created.revision,
            title: "Ship the plan",
            estimate: 3,
        });
        expect(updated.estimate).toBe(3);
        expect([updated.title, updated.revision]).toEqual(["Ship the plan", 2]);
        await expect(
            execute("update", {
                id: created.id,
                requestId: RequestId.create(),
                revision: created.revision,
                title: "Stale",
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "task revision has changed" });

        // keep a sensitive field in the database, out of every row a caller reads and its schema
        await database
            .update(task.table)
            .set({ token: "secret" })
            .where(eq(task.table.id, created.id));
        const read = await execute("get", { id: created.id });
        expect(["token" in read, Object.hasOwn(task.schema.row.shape, "token")]).toEqual([
            false,
            false,
        ]);

        // audit each change once
        expect(await events()).toEqual(["task.create", "task.update", "task.update CONFLICT"]);
    },
);

test.each(TEST_DIALECTS)(
    "share an object, challenge for stronger authentication, accept proposals and lend authority to agents on %s",
    async (dialect) => {
        const { execute, events, act, step, delegate } = await serveTasks(dialect);
        const created = await execute("create", {
            requestId: RequestId.create(),
            title: "Ship the plan",
        });

        // share the task with another user as a viewer, list it, and revoke it again
        const viewer = principal.user.reference("universe", "user-2");
        const shared = await execute("grant", {
            id: created.id,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: viewer,
        });
        expect(await execute("relationships", { id: created.id })).toEqual({
            items: [
                {
                    id: shared.id,
                    object: {
                        packageId: task.policy.definition.packageId,
                        type: "task",
                        scope: spaceId,
                        id: created.id,
                    },
                    relation: "viewer",
                    subject: viewer,
                    createdAt: any(Number),
                    expiresAt: null,
                },
            ],
            cursor: null,
        });
        act("user-2");
        expect((await execute("get", { id: created.id })).title).toBe("Ship the plan");
        await expect(
            execute("grant", {
                id: created.id,
                requestId: RequestId.create(),
                relation: "viewer",
                subject: { ...viewer, id: "user-3" },
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: write" });
        act("user-1");
        await execute("revoke", {
            id: created.id,
            requestId: RequestId.create(),
            relationshipId: shared.id,
        });
        act("user-2");
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${created.id}`,
        });
        act("user-1");

        // challenge a viewer for step-up authentication, then admit them
        await execute("grant", {
            id: created.id,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: { ...viewer, id: "user-3" },
            conditions: { assurance: 2 },
        });
        act("user-3");
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "INSUFFICIENT_AUTHENTICATION",
            message: "authenticate again at the required assurance",
            stepUp: { assurance: 2 },
        });
        step(2);
        expect((await execute("get", { id: created.id })).title).toBe("Ship the plan");
        step(1);

        // let a user without access ask for it, and apply it once the owner accepts
        act("user-4");
        const asked = await execute("propose", {
            id: created.id,
            requestId: RequestId.create(),
            relationship: { relation: "viewer", subject: { ...viewer, id: "user-4" } },
            purpose: "follow the plan",
        });
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${created.id}`,
        });
        await expect(
            execute("accept", {
                id: created.id,
                requestId: RequestId.create(),
                proposalId: asked.id,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: write" });
        act("user-1");
        expect(await execute("proposals", { id: created.id })).toEqual({
            items: [
                {
                    id: asked.id,
                    relationship: {
                        object: {
                            packageId: task.policy.definition.packageId,
                            type: "task",
                            scope: spaceId,
                            id: created.id,
                        },
                        relation: "viewer",
                        expiresAt: null,
                        subject: { ...viewer, id: "user-4" },
                    },
                    proposer: { ...viewer, id: "user-4" },
                    purpose: "follow the plan",
                    createdAt: any(Number),
                    expiresAt: any(Number),
                },
            ],
            cursor: null,
        });
        await execute("accept", {
            id: created.id,
            requestId: RequestId.create(),
            proposalId: asked.id,
        });
        act("user-4");
        expect((await execute("get", { id: created.id })).title).toBe("Ship the plan");
        act("user-1");

        // challenge a lent agent for a missing grant, then admit it
        const agent = principal.installation.reference(spaceId, "assistant");
        delegate(agent);
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "INSUFFICIENT_GRANT",
            message: "propose the missing delegation to the principal the delegate acts for",
            data: { delegate: agent, onBehalfOf: principal.user.reference("universe", "user-1") },
        });
        const owner = principal.user.reference("universe", "user-1");
        const lending = await execute("propose", {
            id: created.id,
            requestId: RequestId.create(),
            relationship: {
                relation: "viewer",
                subject: principal.installation.reference(spaceId, "assistant"),
                conditions: { onBehalfOf: owner },
            },
            purpose: "summarize the plan",
        });
        expect(lending.proposer).toEqual(agent);
        await expect(
            execute("accept", {
                id: created.id,
                requestId: RequestId.create(),
                proposalId: lending.id,
            }),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "only a principal may lend its own authority",
        });
        delegate(undefined);
        await execute("accept", {
            id: created.id,
            requestId: RequestId.create(),
            proposalId: lending.id,
        });
        delegate(agent);
        expect((await execute("get", { id: created.id })).title).toBe("Ship the plan");
        await expect(
            execute("update", {
                id: created.id,
                requestId: RequestId.create(),
                revision: 1,
                title: "Taken",
            }),
        ).rejects.toMatchObject({
            code: "INSUFFICIENT_GRANT",
            message: "propose the missing delegation to the principal the delegate acts for",
        });
        delegate(undefined);

        // audit each change once
        expect(await events()).toEqual([
            "task.create",
            "task.grant",
            "task.revoke",
            "task.grant",
            "task.propose",
            "task.accept",
            "task.propose",
            "task.accept",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "hide guarded fields from readers without their permission, and run each transition once on %s",
    async (dialect) => {
        const { execute, events, act } = await serveTasks(dialect);
        const created = await execute("create", {
            requestId: RequestId.create(),
            title: "Write the plan",
        });
        await execute("update", {
            id: created.id,
            requestId: RequestId.create(),
            revision: created.revision,
            title: "Ship the plan",
            estimate: 3,
        });
        const viewer = principal.user.reference("universe", "user-4");
        await execute("grant", {
            id: created.id,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: viewer,
        });

        // hide the guarded estimate and refuse setting it
        act("user-4");
        const viewed = await execute("get", { id: created.id });
        expect([viewed.title, "estimate" in viewed]).toEqual(["Ship the plan", false]);
        expect((await execute("list", {})).items.map((item) => "estimate" in item)).toEqual([
            false,
        ]);
        act("user-1");
        await execute("grant", {
            id: created.id,
            requestId: RequestId.create(),
            relation: "editor",
            subject: { ...viewer, id: "user-5" },
        });
        act("user-5");
        const edited = await execute("update", {
            id: created.id,
            requestId: RequestId.create(),
            revision: 2,
            title: "Ship the plan",
        });
        expect([edited.revision, "estimate" in edited]).toEqual([3, false]);
        await expect(
            execute("update", {
                id: created.id,
                requestId: RequestId.create(),
                revision: 3,
                title: "Ship the plan",
                estimate: 8,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "field estimate is not writable" });
        act("user-1");

        // complete the task once and refuse a second and a guarded completion
        const completed = await execute("complete", {
            id: created.id,
            requestId: RequestId.create(),
        });
        expect(completed.status).toBe("done");
        await expect(
            execute("complete", { id: created.id, requestId: RequestId.create() }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "task cannot complete while status is done",
        });
        await execute("reopen", { id: created.id, requestId: RequestId.create() });

        // audit each change once
        expect(await events()).toEqual([
            "task.create",
            "task.update",
            "task.grant",
            "task.grant",
            "task.update",
            "task.complete",
            "task.complete CONFLICT",
            "task.reopen",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse changing objects a stack manages, then trash, restore and purge them, the expired ones as the system, on %s",
    async (dialect) => {
        const { server, database, execute, events, purges } = await serveTasks(dialect);
        const created = await execute("create", {
            requestId: RequestId.create(),
            title: "Ship the plan",
        });

        // refuse standard changes to a managed task
        const archived = await execute("archive", {
            id: created.id,
            requestId: RequestId.create(),
        });
        expect(archived.archived).toBe(true);
        await database
            .update(task.table)
            .set({
                managerInstallationId: schema
                    .identifier("installation")
                    .parse("installation-01996ab0-0000-7000-8000-000000000002"),
                managerPackageId: schema
                    .identifier("package")
                    .parse("package-01996ab0-0000-7000-8000-000000000003"),
                managerName: "plan",
            })
            .where(eq(task.table.id, created.id));
        await expect(
            execute("delete", {
                id: created.id,
                requestId: RequestId.create(),
                revision: archived.revision,
            }),
        ).rejects.toMatchObject({
            code: "MANAGED",
            message: "task is managed by its stack; detach it before changing it",
        });

        // delete once detached: hidden from lists, listed in the trash, restorable, then purged
        await database
            .update(task.table)
            .set({ detachedAt: Date.now() })
            .where(eq(task.table.id, created.id));
        await execute("delete", {
            id: created.id,
            requestId: RequestId.create(),
            revision: archived.revision,
        });
        const titles = async (deleted: "exclude" | "only") =>
            (await execute("list", { deleted })).items.map((item) => item.title);
        expect([await titles("exclude"), await titles("only")]).toEqual([[], ["Ship the plan"]]);
        await execute("restore", { id: created.id, requestId: RequestId.create() });
        expect(await titles("exclude")).toEqual(["Ship the plan"]);
        const restored = await execute("get", {
            id: created.id,
        });
        await execute("delete", {
            id: created.id,
            requestId: RequestId.create(),
            revision: restored.revision,
        });
        await execute("purge", { id: created.id, requestId: RequestId.create() });
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${created.id}`,
        });

        // purge the trash once the recovery window passes
        const stale = await execute("create", {
            requestId: RequestId.create(),
            title: "Old",
        });
        await execute("delete", {
            id: stale.id,
            requestId: RequestId.create(),
            revision: stale.revision,
        });
        expect(await recoverable.purge(server, Date.now() + 60_000)).toBe(1);

        // purge more trash than one batch takes, across several transactions
        const trashed = Array.from({ length: 100 }, () => ({
            ...stale,
            id: schema.identifier("task").parse(`task-${v7()}`),
            deletionRequestedAt: Date.now(),
        }));
        await database.insert(task.table).values(trashed);
        expect(await recoverable.purge(server, Date.now() + 60_000)).toBe(100);

        // audit each purge as the system's, and each caller's change once
        expect(await purges()).toEqual(Array.from({ length: 101 }, () => "task.purge"));
        expect(await events()).toEqual([
            "task.create",
            "task.archive",
            "task.delete MANAGED",
            "task.delete",
            "task.restore",
            "task.delete",
            "task.purge",
            "task.create",
            "task.delete",
        ]);
    },
);

test("prepare external work after the access check, settle it after commit, compensate on failure and skip it on replay", async () => {
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        objectDatabase,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // archive through an external system by exporting
    const external: string[] = [];
    let isFailing = false;
    const handled = task.handle({
        export: {
            prepare: async (call) => {
                if (call.target.title === "Broken") {
                    throw new Error("reservation failed");
                }
                external.push(`reserve ${call.target.title}`);

                return `reservation-${call.target.id}`;
            },
            handler: async ({ target, database, prepared }) => {
                if (isFailing) {
                    throw new Error("archive failed");
                }
                const [row] = await database
                    .update(task.table)
                    .set({
                        archived: true,
                        title: `${target.title} (${prepared})`,
                        revision: target.revision + 1,
                    })
                    .where(eq(task.table.id, target.id))
                    .returning();

                return present(row, "the exported task");
            },
            commit: async (_call, prepared) => {
                external.push(`confirm ${prepared}`);
            },
            rollback: async (_call, prepared) => {
                external.push(`release ${String(prepared)}`);
            },
        },
    });
    const server = new ObjectServer({
        objects: { task: handled, comment, taskVersion, folder },
        database: storage.database,
        callKey: testCallKey,
        origin: {
            package: task.package,
            service: "test",
        },
    });
    let context = userContext("user-1", spaceId);
    const execute = <Name extends CallableName<typeof handled>>(
        name: Name,
        input: Record<string, unknown>,
    ) => server.call(handled, name, { spaceId, ...input }, context);
    const created = await execute("create", { requestId: RequestId.create(), title: "Plan" });
    const reservation = `reservation-${created.id}`;

    // refuse a caller without the permission before any external work
    context = userContext("user-2", spaceId);
    await expect(
        execute("export", { requestId: RequestId.create(), id: created.id }),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no task ${created.id}` });
    expect(external).toEqual([]);

    // release the prepared work and leave the task unchanged when the transaction fails
    context = userContext("user-1", spaceId);
    isFailing = true;
    await expect(
        execute("export", { requestId: RequestId.create(), id: created.id }),
    ).rejects.toThrow(new Error("archive failed"));
    expect(external).toEqual(["reserve Plan", `release ${reservation}`]);

    // confirm after commit and replay without repeating the work
    isFailing = false;
    external.length = 0;
    const exported = { requestId: RequestId.create(), id: created.id };
    const archived = await execute("export", exported);
    const author = Subject.key(principal.user.reference("universe", "user-1"));
    expect(archived).toEqual({
        id: created.id,
        createdAt: any(Number),
        createdBy: author,
        updatedAt: any(Number),
        updatedBy: author,
        deletedBy: null,
        revision: 2,
        tags: {},
        scope: spaceId,
        deletionRequestedAt: null,
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
        ownerId: "user-1",
        title: `Plan (${reservation})`,
        archived: true,
        estimate: null,
        origin: null,
        currentId: null,
        status: "open",
    });
    expect(await execute("export", exported)).toEqual(archived);
    expect(external).toEqual(["reserve Plan", `confirm ${reservation}`]);

    // release the work prepared for earlier calls and the failed preparation's own once a later call's preparation fails
    const other = await execute("create", { requestId: RequestId.create(), title: "Other" });
    const broken = await execute("create", {
        requestId: RequestId.create(),
        title: "Broken",
    });
    external.length = 0;
    await expect(
        server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    Call.record(handled, "export", { spaceId, id: other.id }),
                    Call.record(handled, "export", { spaceId, id: broken.id }),
                ],
            },
            context,
        ),
    ).rejects.toThrow(new Error("reservation failed"));
    expect(external).toEqual([
        "reserve Other",
        `release reservation-${other.id}`,
        "release undefined",
    ]);

    // refuse a mutation acting in two scopes
    external.length = 0;
    const elsewhere = schema.identifier("space").parse(`space-${v7()}`);
    await expect(
        server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    Call.record(handled, "export", { spaceId, id: other.id }),
                    Call.record(handled, "export", { spaceId: elsewhere, id: other.id }),
                ],
            },
            userContext("user-1", undefined),
        ),
    ).rejects.toMatchObject({
        code: "BAD_REQUEST",
        message: "a mutation's calls act in one scope",
    });
    expect(external).toEqual([]);
});

test("authorize a creation before its external work, and settle committed work through the settlement controller once settling fails", async () => {
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        objectDatabase,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // record each version's external copy, failing the first confirmation after commit
    const external: string[] = [];
    let isSettling = false;
    const copies = taskCopy.handle({
        create: {
            prepare: async (call) => {
                external.push(`copy ${call.input["title"]}`);

                return `copy-${present(call.id, "the version's identifier")}`;
            },
            commit: async (_call, prepared) => {
                if (!isSettling) {
                    isSettling = true;
                    throw new Error("confirmation failed");
                }
                external.push(`confirm ${prepared}`);
            },
            rollback: async (_call, prepared) => {
                external.push(`release ${String(prepared)}`);
            },
        },
    });
    const reported: unknown[] = [];
    const server = new ObjectServer({
        objects: { task, comment, taskVersion, taskCopy: copies, folder },
        database: storage.database,
        callKey: testCallKey,
        origin: {
            package: task.package,
            service: "test",
        },
        report: (error) => reported.push(error),
    });
    let context = userContext("user-1", spaceId);
    const created = await server.call(
        task,
        "create",
        { spaceId, requestId: RequestId.create(), title: "Plan" },
        context,
    );

    // refuse a caller who may not write the task before any external work
    context = userContext("user-2", spaceId);
    const version = { spaceId, parentId: created.id, title: "Draft" };
    await expect(
        server.call(copies, "create", { ...version, requestId: RequestId.create() }, context),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no task ${created.id}` });
    expect(external).toEqual([]);

    // commit and keep the settlement despite a failed confirmation
    context = userContext("user-1", spaceId);
    const drafted = await server.call(
        copies,
        "create",
        { ...version, requestId: RequestId.create() },
        context,
    );
    const kept = await storage.database.select().from(settlement);
    expect([external, kept.map((row) => [row.method, row.target]), reported]).toEqual([
        ["copy Draft"],
        [["create", drafted.id]],
        [new Error("confirmation failed")],
    ]);

    // confirm it through the controller
    const controller = Settlement.controller(server, { grace: { milliseconds: 0 } });
    expect(
        await controller.reconcile(aligned(kept, 0).id, reconciliation(AbortSignal.timeout(5000))),
    ).toBeUndefined();
    expect([external, await storage.database.select().from(settlement)]).toEqual([
        ["copy Draft", `confirm copy-${drafted.id}`],
        [],
    ]);
});

test("cancel a call that outlasts its settlement grace by its key, and refuse to commit it", async () => {
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        objectDatabase,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // keep external copies by call key, pausing mid-copy
    const external: string[] = [];
    let during: () => Promise<void> = idle;
    const copies = taskCopy.handle({
        create: {
            prepare: async (call) => {
                external.push(`copy ${call.idempotencyKey === undefined ? "unkeyed" : "keyed"}`);
                await during();

                return present(call.idempotencyKey, "the copy's key");
            },
            commit: async () => {
                external.push("confirm prepared");
            },
            rollback: async (_call, prepared) => {
                external.push(`cancel ${prepared === undefined ? "by key" : "prepared"}`);
            },
        },
    });
    const server = new ObjectServer({
        objects: { task, comment, taskVersion, taskCopy: copies, folder },
        database: storage.database,
        callKey: testCallKey,
        origin: {
            package: task.package,
            service: "test",
        },
    });
    const context = userContext("user-1", spaceId);
    const created = await server.call(
        task,
        "create",
        { spaceId, requestId: RequestId.create(), title: "Plan" },
        context,
    );

    // cancel the reserved settlement mid-copy
    const controller = Settlement.controller(server, { grace: { milliseconds: 0 } });
    during = async () => {
        const [reserved] = await storage.database.select().from(settlement);
        await controller.reconcile(
            present(reserved, "the reserved settlement").id,
            reconciliation(AbortSignal.timeout(5000)),
        );
    };
    const version = {
        spaceId,
        parentId: created.id,
        title: "Draft",
        requestId: RequestId.create(),
    };
    await expect(server.call(copies, "create", version, context)).rejects.toMatchObject({
        code: "SERVICE_UNAVAILABLE",
        message: "task-copy.create outlasted its settlement grace",
    });

    // cancel it once by key
    expect([
        external,
        await storage.database.select().from(settlement),
        await storage.database.select().from(taskCopy.table),
    ]).toEqual([["copy keyed", "cancel by key"], [], []]);
});

test("predict custom methods only where their handler is, and never a scope's suspension", () => {
    // predict only methods with a handler on the client
    const unhandled = method.mutation({ permission: "archive" });
    const handled = unhandled.handle(async (call) => call.target);
    expect([
        unhandled.isPredicted,
        handled.isPredicted,
        suspendable.suspend("update").isPredicted,
        suspendable.resume("update").isPredicted,
    ]).toEqual([false, true, false, false]);
});

test("add suspending and detaching through their traits, and refuse them where the trait does not apply", () => {
    // derive suspend and resume on a scope type, and detach on a declarable type
    const room = defineObject({
        name: "room",
        plural: "rooms",
        scope: "universe",
        isScope: true,
        fields: {},
        permissions: ["read", "update"],
        suspendable: { by: "update" },
        methods: (builder) => ({ get: builder.get("read") }),
    });
    const card = defineObject({
        name: "card",
        plural: "cards",
        scope: room,
        declarable: { schema: schema.object({}) },
        fields: {},
        permissions: ["read", "write"],
        detachable: { by: "write" },
        methods: (builder) => ({ get: builder.get("read") }),
    });
    expect([Object.keys(room.methods).toSorted(), Object.keys(card.methods).toSorted()]).toEqual([
        ["get", "resume", "suspend"],
        ["detach", "get"],
    ]);

    // refuse suspending objects that are no scope, and detaching records no stack declares
    const plain = { plural: "items", scope: room, fields: {}, permissions: ["update"] } as const;
    expect(() => defineObject({ ...plain, name: "desk", suspendable: { by: "update" } })).toThrow(
        "object desk is suspendable but no scope",
    );
    expect(() => defineObject({ ...plain, name: "lamp", detachable: { by: "update" } })).toThrow(
        "object lamp is detachable but not declarable",
    );
});

/** Serve tasks, comments, versions and folders in a space to a scenario's caller. */
async function serveTasks(dialect: Dialect) {
    // keep the space's objects in a migrated database
    const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // handle archiving
    const handled = task.handle({
        archive: async ({ target, database: transaction }) => {
            const [row] = await transaction
                .update(task.table)
                .set({ archived: true, revision: target.revision + 1 })
                .where(eq(task.table.id, target.id))
                .returning();

            return present(row, "the archived task");
        },
    });

    // decide as the current user at the current assurance, through the current delegate
    let current = "user-1";
    let level = 1;
    let agent: Subject | undefined;
    const sign = () =>
        userContext(current, spaceId, {
            assurance: { level, authenticatedAt: Date.now() },
            ...(agent === undefined ? {} : { delegates: [{ subject: agent, authority: "lent" }] }),
        });
    let context = sign();

    // serve the objects, recording each audited change in the outbox
    const server = new ObjectServer({
        objects: { task: handled, comment, taskVersion, folder },
        database,
        callKey: testCallKey,
        origin: {
            package: task.package,
            service: "test",
        },
    });

    return {
        server,
        database,
        /** Read the callers' audited actions. */
        events: () => auditedActions(database, "caller"),
        /** Read the system's audited actions. */
        purges: () => auditedActions(database, "system"),
        /** Read the watermarks the current caller's requests reached. */
        observed: () => context.observed,
        /** Execute a task method as the current caller. */
        execute: <Name extends CallableName<typeof handled>>(
            name: Name,
            input: Record<string, unknown>,
        ) => server.call(handled, name, { spaceId, ...input }, context),
        /** Act as a user. */
        act: (user: string) => {
            current = user;
            context = sign();
        },
        /** Authenticate at an assurance level. */
        step: (assurance: number) => {
            level = assurance;
            context = sign();
        },
        /** Act through a delegate, or directly. */
        delegate: (delegate: Subject | undefined) => {
            agent = delegate;
            context = sign();
        },
    };
}

test("leave standard methods to the server where the declaration says so", () => {
    // predict standard methods by default, and not those only a server handler completes
    expect([
        method.create("write").isPredicted,
        method.create("write", { isPredicted: false }).isPredicted,
        method.update("write", { isPredicted: false }).isPredicted,
    ]).toEqual([true, false, false]);
});

test("carry a key the method derives from its work, the same for every retry of it", async () => {
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        objectDatabase,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // copy each version under a key of its title, failing the first attempt after the copy
    const keys: string[] = [];
    let isFailing = true;
    const copies = taskCopy.handle({
        create: {
            idempotencyKey: (call) => `copy-${call.input["title"]}`,
            prepare: async (call) => {
                const key = present(call.idempotencyKey, "the call's key");
                keys.push(key);
                if (isFailing) {
                    isFailing = false;
                    throw new ServiceError("UNAVAILABLE", { message: "copy failed" });
                }

                return key;
            },
            rollback: async () => {},
        },
    });
    const server = new ObjectServer({
        objects: { task, comment, taskVersion, taskCopy: copies, folder },
        database: storage.database,
        callKey: testCallKey,
        origin: {
            package: task.package,
            service: "test",
        },
    });
    const context = userContext("user-1", spaceId);
    const created = await server.call(
        task,
        "create",
        { spaceId, requestId: RequestId.create(), title: "Plan" },
        context,
    );

    // retry the failed creation under the same request with the same key both times
    const version = {
        spaceId,
        parentId: created.id,
        title: "Draft",
        requestId: RequestId.create(),
    };
    await expect(server.call(copies, "create", version, context)).rejects.toMatchObject({
        code: "UNAVAILABLE",
    });
    await server.call(copies, "create", version, context);
    expect(keys).toEqual(["copy-Draft", "copy-Draft"]);
});

test("settle each call's prepared work once when two calls of one mutation share an idempotency key", async () => {
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        objectDatabase,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // copy under a key of the title, confirming each copy
    const confirmed: unknown[] = [];
    const copies = taskCopy.handle({
        create: {
            idempotencyKey: (call) => `copy-${call.input["title"]}`,
            prepare: async (call) => present(call.idempotencyKey, "the call's key"),
            commit: async (_call, prepared) => {
                confirmed.push(prepared);
            },
        },
    });
    const server = new ObjectServer({
        objects: { task, comment, taskVersion, taskCopy: copies, folder },
        database: storage.database,
        callKey: testCallKey,
        origin: {
            package: task.package,
            service: "test",
        },
    });
    const context = userContext("user-1", spaceId);
    const created = await server.call(
        task,
        "create",
        { spaceId, requestId: RequestId.create(), title: "Plan" },
        context,
    );

    // create two copies of one title in one mutation, confirming both and keeping no settlement
    const copy = { spaceId, parentId: created.id, title: "Draft" };
    await server.mutate(
        {
            id: RequestId.create(),
            calls: [Call.record(copies, "create", copy), Call.record(copies, "create", copy)],
        },
        context,
    );
    expect([confirmed, await storage.database.select().from(settlement)]).toEqual([
        ["copy-Draft", "copy-Draft"],
        [],
    ]);
});

/** Wait for nothing, until a test pauses a call in its place. */
async function idle(): Promise<void> {}
