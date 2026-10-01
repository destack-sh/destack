import { AuditOutbox } from "@destack/audit/outbox";
import { reconciliation, testJournalKey } from "@destack/service/test";
import { expect, onTestFinished, test } from "@destack/test";
import { ServiceError } from "@destack/service/error";
import type { Subject } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { type Dialect, eq } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier, schema } from "@destack/schema";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import { Bookmark } from "@destack/service/bookmark";
import type { ServiceContext } from "@destack/service/server";
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
import { comment, folder, objectDatabase, request, task, taskVersion } from "./schema.ts";
import { principal } from "@destack/access";
import { openSpace, space } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";

/** The space containing the tasks. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

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
    const _update: UpdateInput = {
        spaceId,
        id: identifier("task").parse("task-01996ab0-0000-7000-8000-000000000004"),
        requestId: RequestId.create(),
        revision: 1,
        title: "Typed",
    };
    // @ts-expect-error updates require a request identifier
    const _unnamed: UpdateInput = { spaceId, id: _update.id, title: "x" };
    const _title: GetOutput["title"] = "typed";

    // derive one route per method under the object's scope
    const procedures = task.procedures as Record<
        string,
        { "~orpc": { route: { method: string; path: string } } }
    >;
    expect(
        Object.fromEntries(
            Object.entries(procedures).map(([name, procedure]) => [
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
    expect(description.methods.archive).toEqual({
        kind: "custom",
        permission: "write",
        mutates: true,
        isPredicted: false,
        route: { method: "POST", path: "/spaces/{spaceId}/tasks/{id}/archive" },
        input: expect.any(Object),
        output: { kind: "value", schema: expect.any(Object) },
        audit: {
            name: "Task.archive",
            package: {
                id: task.policy.definition.packageId,
                name: "@destack/object",
                version: expect.any(String),
            },
            targets: expect.any(Object),
            details: expect.any(Object),
        },
    });

    // take every writable field in an update, requiring only the target and request
    const updateInput = description.methods.update!.input as {
        properties: Record<string, unknown>;
        required: string[];
    };
    expect([Object.keys(updateInput.properties), updateInput.required]).toEqual([
        ["spaceId", "id", "requestId", "revision", "title", "estimate", "current"],
        ["spaceId", "id", "requestId"],
    ]);
    expect(description.methods.get!.audit).toBeUndefined();
});

test.each(TEST_DIALECTS)(
    "route every served object, create once per request identifier and take chosen identifiers on %s",
    async (dialect) => {
        const { server, database, execute, events, observed, as } = await serveTasks(dialect);

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
                    context: () => as("user-1"),
                    journal: new Journal(request, testJournalKey),
                    audit: AuditRecorder.service(new AuditOutbox(database), {
                        package: task.package,
                        service: "test",
                    }),
                }),
        ).toThrow(new TypeError("object replica takes the name of the shared replica procedures"));

        // create once per request identifier, replaying the original result
        const creation = { requestId: RequestId.create(), title: "Write the plan" };
        const created = (await execute("create", creation)) as typeof task.table.$inferSelect;
        expect(observed.watermarks.at(-1)).toEqual({
            scope: spaceId,
            epoch: await database.log.epoch(),
            sequence: expect.any(Number),
        });
        expect(await execute("create", creation)).toEqual(created);

        // take a chosen identifier and refuse a taken one
        const chosen = identifier("task").parse(`task-${v7()}`);
        const drafted = { requestId: RequestId.create(), id: chosen, title: "Draft" };
        expect(((await execute("create", drafted)) as { id: string }).id).toBe(chosen);
        await expect(
            execute("create", { requestId: RequestId.create(), id: chosen, title: "Again" }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "task identifier is taken" });
        const ahead = identifier("task").parse(`task-${v7({ msecs: Date.now() + 3_600_000 })}`);
        const later = { requestId: RequestId.create(), id: ahead, title: "Later" };
        expect(((await execute("create", later)) as { id: string }).id).toBe(ahead);
        await database.delete(task.table).where(eq(task.table.id, chosen));
        await database.delete(task.table).where(eq(task.table.id, ahead));
        expect(
            ((await execute("list", {})) as { items: unknown[] }).items.map(
                (item) => (item as { title: string }).title,
            ),
        ).toEqual(["Write the plan"]);

        // audit each creation once
        expect(await events()).toEqual([
            "Task.create",
            "Task.create",
            "Task.create CONFLICT",
            "Task.create",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "hide an object from strangers, update it at the observed revision, and keep sensitive fields out of rows on %s",
    async (dialect) => {
        const { database, execute, events, act } = await serveTasks(dialect);
        const created = (await execute("create", {
            requestId: RequestId.create(),
            title: "Write the plan",
        })) as typeof task.table.$inferSelect;

        // hide the task from another user, and refuse them changes without revealing it
        act("user-2");
        expect(((await execute("list", {})) as { items: unknown[] }).items).toEqual([]);
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
        const updated = (await execute("update", {
            id: created.id,
            requestId: RequestId.create(),
            revision: created.revision,
            title: "Ship the plan",
            estimate: 3,
        })) as { title: string; revision: number; estimate?: number };
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
        const read = (await execute("get", { id: created.id })) as Record<string, unknown>;
        expect(["token" in read, Object.hasOwn(task.schema.row.shape, "token")]).toEqual([
            false,
            false,
        ]);

        // audit each change once
        expect(await events()).toEqual(["Task.create", "Task.update", "Task.update CONFLICT"]);
    },
);

test.each(TEST_DIALECTS)(
    "share an object, challenge for stronger authentication, accept proposals and lend authority to agents on %s",
    async (dialect) => {
        const { execute, events, act, step, delegate } = await serveTasks(dialect);
        const created = (await execute("create", {
            requestId: RequestId.create(),
            title: "Ship the plan",
        })) as typeof task.table.$inferSelect;

        // share the task with another user as a viewer, list it, and revoke it again
        const viewer = principal.user.reference("universe", "user-2");
        const shared = (await execute("grant", {
            id: created.id,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: viewer,
        })) as { id: string };
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
                    createdAt: expect.any(Number),
                    expiresAt: null,
                },
            ],
            cursor: null,
        });
        act("user-2");
        expect(((await execute("get", { id: created.id })) as { title: string }).title).toBe(
            "Ship the plan",
        );
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
        expect(((await execute("get", { id: created.id })) as { title: string }).title).toBe(
            "Ship the plan",
        );
        step(1);

        // let a user without access ask for it, and apply it once the owner accepts
        act("user-4");
        const asked = (await execute("propose", {
            id: created.id,
            requestId: RequestId.create(),
            relationship: { relation: "viewer", subject: { ...viewer, id: "user-4" } },
            purpose: "follow the plan",
        })) as { id: string };
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
                    createdAt: expect.any(Number),
                    expiresAt: expect.any(Number),
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
        expect(((await execute("get", { id: created.id })) as { title: string }).title).toBe(
            "Ship the plan",
        );
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
        const lending = (await execute("propose", {
            id: created.id,
            requestId: RequestId.create(),
            relationship: {
                relation: "viewer",
                subject: principal.installation.reference(spaceId, "assistant"),
                conditions: { onBehalfOf: owner },
            },
            purpose: "summarize the plan",
        })) as { id: string; proposer: Subject };
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
        expect(((await execute("get", { id: created.id })) as { title: string }).title).toBe(
            "Ship the plan",
        );
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
            "Task.create",
            "Task.grant",
            "Task.revoke",
            "Task.grant",
            "Task.propose",
            "Task.accept",
            "Task.propose",
            "Task.accept",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "hide guarded fields from readers without their permission, and run each transition once on %s",
    async (dialect) => {
        const { execute, events, act } = await serveTasks(dialect);
        const created = (await execute("create", {
            requestId: RequestId.create(),
            title: "Write the plan",
        })) as typeof task.table.$inferSelect;
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
        const viewed = (await execute("get", { id: created.id })) as Record<string, unknown>;
        expect([viewed.title, "estimate" in viewed]).toEqual(["Ship the plan", false]);
        expect(
            ((await execute("list", {})) as { items: Record<string, unknown>[] }).items.map(
                (item) => "estimate" in item,
            ),
        ).toEqual([false]);
        act("user-1");
        await execute("grant", {
            id: created.id,
            requestId: RequestId.create(),
            relation: "editor",
            subject: { ...viewer, id: "user-5" },
        });
        act("user-5");
        const edited = (await execute("update", {
            id: created.id,
            requestId: RequestId.create(),
            revision: 2,
            title: "Ship the plan",
        })) as Record<string, unknown>;
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
        const completed = (await execute("complete", {
            id: created.id,
            requestId: RequestId.create(),
        })) as typeof task.table.$inferSelect;
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
            "Task.create",
            "Task.update",
            "Task.grant",
            "Task.grant",
            "Task.update",
            "Task.complete",
            "Task.complete CONFLICT",
            "Task.reopen",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse changing objects a stack manages, then trash, restore and purge them, the expired ones as the system, on %s",
    async (dialect) => {
        const { server, database, execute, events, purges } = await serveTasks(dialect);
        const created = (await execute("create", {
            requestId: RequestId.create(),
            title: "Ship the plan",
        })) as typeof task.table.$inferSelect;

        // refuse standard changes to a managed task
        const archived = (await execute("archive", {
            id: created.id,
            requestId: RequestId.create(),
        })) as { archived: boolean; revision: number };
        expect(archived.archived).toBe(true);
        await database
            .update(task.table)
            .set({
                managerInstallationId: identifier("installation").parse(
                    "installation-01996ab0-0000-7000-8000-000000000002",
                ),
                managerPackageId: identifier("package").parse(
                    "package-01996ab0-0000-7000-8000-000000000003",
                ),
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
            ((await execute("list", { deleted })) as { items: { title: string }[] }).items.map(
                (item) => item.title,
            );
        expect([await titles("exclude"), await titles("only")]).toEqual([[], ["Ship the plan"]]);
        await execute("restore", { id: created.id, requestId: RequestId.create() });
        expect(await titles("exclude")).toEqual(["Ship the plan"]);
        const restored = (await execute("get", {
            id: created.id,
        })) as typeof task.table.$inferSelect;
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
        const stale = (await execute("create", {
            requestId: RequestId.create(),
            title: "Old",
        })) as typeof task.table.$inferSelect;
        await execute("delete", {
            id: stale.id,
            requestId: RequestId.create(),
            revision: stale.revision,
        });
        expect(await recoverable.purge(server, Date.now() + 60_000)).toBe(1);

        // purge more trash than one batch takes, across several transactions
        const trashed = Array.from({ length: 100 }, () => ({
            ...stale,
            id: identifier("task").parse(`task-${v7()}`),
            deletionRequestedAt: Date.now(),
        }));
        await database.insert(task.table).values(trashed);
        expect(await recoverable.purge(server, Date.now() + 60_000)).toBe(100);

        // audit each purge as the system's, and each caller's change once
        expect(await purges()).toEqual(Array.from({ length: 101 }, () => "Task.purge"));
        expect(await events()).toEqual([
            "Task.create",
            "Task.archive",
            "Task.delete MANAGED",
            "Task.delete",
            "Task.restore",
            "Task.delete",
            "Task.purge",
            "Task.create",
            "Task.delete",
        ]);
    },
);

test("prepare external work after the access check, settle it after commit, compensate on failure and skip it on replay", async () => {
    const storage = await TestDatabase.create(TEST_DIALECTS.at(-1)!, objectDatabase, {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // archive through an external system
    const external: string[] = [];
    let isFailing = false;
    const handled = task.handle({
        archive: {
            prepare: async (call) => {
                if (call.target!.title === "Broken") {
                    throw new Error("reservation failed");
                }
                external.push(`reserve ${call.target!.title}`);

                return `reservation-${call.target!.id}`;
            },
            effect: async ({ target, database, prepared }) => {
                if (isFailing) {
                    throw new Error("archive failed");
                }
                const [row] = await database
                    .update(task.table)
                    .set({
                        archived: true,
                        title: `${target!.title} (${String(prepared)})`,
                        revision: target!.revision + 1,
                    })
                    .where(eq(task.table.id, target!.id))
                    .returning();

                return row;
            },
            settle: async (_call, prepared, isCommitted) => {
                external.push(`${isCommitted ? "confirm" : "release"} ${String(prepared)}`);
            },
        },
    });
    let current = "user-1";
    const server = new ObjectServer({
        objects: { task: handled, comment, taskVersion, folder },
        database: storage.database,
        context: () => ({
            subjects: [principal.user.reference("universe", current)],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(request, testJournalKey),
        audit: AuditRecorder.service(new AuditOutbox(storage.database), {
            package: task.package,
            service: "test",
        }),
    });
    const context = {
        scope: spaceId,
        requireCaller: () => ({ id: current }),
        bookmark: new Bookmark(),
        observed: new Bookmark(),
    } as unknown as ServiceContext;
    const execute = (name: string, input: Record<string, unknown>) =>
        server.call(handled, name, { spaceId, ...input }, context);
    const created = (await execute("create", { requestId: RequestId.create(), title: "Plan" })) as {
        id: string;
    };
    const reservation = `reservation-${created.id}`;

    // refuse a caller without the permission before any external work
    current = "user-2";
    await expect(
        execute("archive", { requestId: RequestId.create(), id: created.id }),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no task ${created.id}` });
    expect(external).toEqual([]);

    // release the prepared work and leave the task unchanged when the transaction fails
    current = "user-1";
    isFailing = true;
    await expect(
        execute("archive", { requestId: RequestId.create(), id: created.id }),
    ).rejects.toThrow(new Error("archive failed"));
    expect(external).toEqual(["reserve Plan", `release ${reservation}`]);

    // confirm after commit and replay without repeating the work
    isFailing = false;
    external.length = 0;
    const archive = { requestId: RequestId.create(), id: created.id };
    const archived = await execute("archive", archive);
    expect(archived).toEqual({
        id: created.id,
        createdAt: expect.any(Number),
        updatedAt: expect.any(Number),
        revision: 2,
        tags: {},
        scope: spaceId,
        deletionRequestedAt: null,
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
        owner: "user-1",
        title: `Plan (${reservation})`,
        archived: true,
        estimate: null,
        origin: null,
        current: null,
        status: "open",
    });
    expect(await execute("archive", archive)).toEqual(archived);
    expect(external).toEqual(["reserve Plan", `confirm ${reservation}`]);

    // release the work prepared for earlier calls once a later call's preparation fails
    const other = (await execute("create", { requestId: RequestId.create(), title: "Other" })) as {
        id: string;
    };
    const broken = (await execute("create", {
        requestId: RequestId.create(),
        title: "Broken",
    })) as {
        id: string;
    };
    external.length = 0;
    await expect(
        server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    Call.record(handled, "archive", { spaceId, id: other.id }),
                    Call.record(handled, "archive", { spaceId, id: broken.id }),
                ],
            },
            context,
        ),
    ).rejects.toThrow(new Error("reservation failed"));
    expect(external).toEqual(["reserve Other", `release reservation-${other.id}`]);

    // refuse a mutation acting in two scopes
    external.length = 0;
    const elsewhere = identifier("space").parse(`space-${v7()}`);
    await expect(
        server.mutate(
            {
                id: RequestId.create(),
                calls: [
                    Call.record(handled, "archive", { spaceId, id: other.id }),
                    Call.record(handled, "archive", { spaceId: elsewhere, id: other.id }),
                ],
            },
            { ...context, scope: undefined } as ServiceContext,
        ),
    ).rejects.toMatchObject({
        code: "BAD_REQUEST",
        message: "a mutation's calls act in one scope",
    });
    expect(external).toEqual([]);
});

test("authorize a creation before its external work, and settle committed work through the settlement controller once settling fails", async () => {
    const storage = await TestDatabase.create(TEST_DIALECTS.at(-1)!, objectDatabase, {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // record each version's external copy, failing the first confirmation after commit
    const external: string[] = [];
    let isSettling = false;
    const versions = taskVersion.handle({
        create: {
            prepare: async (call) => {
                external.push(`copy ${String(call.input.title)}`);

                return `copy-${call.id!}`;
            },
            settle: async (_call, prepared, isCommitted) => {
                if (isCommitted && !isSettling) {
                    isSettling = true;
                    throw new Error("confirmation failed");
                }
                external.push(`${isCommitted ? "confirm" : "release"} ${String(prepared)}`);
            },
        },
    });
    const reported: unknown[] = [];
    let current = "user-1";
    const server = new ObjectServer({
        objects: { task, comment, taskVersion: versions, folder },
        database: storage.database,
        context: () => ({
            subjects: [principal.user.reference("universe", current)],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(request, testJournalKey),
        audit: AuditRecorder.service(new AuditOutbox(storage.database), {
            package: task.package,
            service: "test",
        }),
        report: (error) => reported.push(error),
    });
    const context = {
        scope: spaceId,
        requireCaller: () => ({ id: current }),
        bookmark: new Bookmark(),
        observed: new Bookmark(),
    } as unknown as ServiceContext;
    const created = (await server.call(
        task,
        "create",
        { spaceId, requestId: RequestId.create(), title: "Plan" },
        context,
    )) as { id: string };

    // refuse a caller who may not write the task before any external work
    current = "user-2";
    const version = { spaceId, parentId: created.id, title: "Draft" };
    await expect(
        server.call(versions, "create", { ...version, requestId: RequestId.create() }, context),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no task ${created.id}` });
    expect(external).toEqual([]);

    // commit and keep the settlement despite a failed confirmation
    current = "user-1";
    const drafted = (await server.call(
        versions,
        "create",
        { ...version, requestId: RequestId.create() },
        context,
    )) as { id: string };
    const kept = await storage.database.select().from(settlement);
    expect([external, kept.map((row) => [row.method, row.target]), reported]).toEqual([
        ["copy Draft"],
        [["create", drafted.id]],
        [new Error("confirmation failed")],
    ]);

    // confirm it through the controller
    const controller = Settlement.controller(server, { grace: { milliseconds: 0 } });
    expect(
        await controller.reconcile(kept[0]!.id, reconciliation(AbortSignal.timeout(5000))),
    ).toBeUndefined();
    expect([external, await storage.database.select().from(settlement)]).toEqual([
        ["copy Draft", `confirm copy-${drafted.id}`],
        [],
    ]);
});

test("cancel a call that outlasts its settlement grace by its key, and refuse to commit it", async () => {
    const storage = await TestDatabase.create(TEST_DIALECTS.at(-1)!, objectDatabase, {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // hold external copies by call key, pausing mid-copy
    const external: string[] = [];
    let during: () => Promise<void> = async () => {};
    const versions = taskVersion.handle({
        create: {
            prepare: async (call) => {
                external.push(`copy ${call.key === undefined ? "unkeyed" : "keyed"}`);
                await during();

                return call.key;
            },
            settle: async (call, prepared, isCommitted) => {
                const held = prepared === undefined ? "by key" : "prepared";
                const verb = isCommitted ? "confirm" : "cancel";
                external.push(`${verb} ${call.key === undefined ? "unkeyed" : held}`);
            },
        },
    });
    const server = new ObjectServer({
        objects: { task, comment, taskVersion: versions, folder },
        database: storage.database,
        context: () => ({
            subjects: [principal.user.reference("universe", "user-1")],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(request, testJournalKey),
        audit: AuditRecorder.service(new AuditOutbox(storage.database), {
            package: task.package,
            service: "test",
        }),
    });
    const context = {
        scope: spaceId,
        requireCaller: () => ({ id: "user-1" }),
        bookmark: new Bookmark(),
        observed: new Bookmark(),
    } as unknown as ServiceContext;
    const created = (await server.call(
        task,
        "create",
        { spaceId, requestId: RequestId.create(), title: "Plan" },
        context,
    )) as { id: string };

    // cancel the reserved settlement mid-copy
    const controller = Settlement.controller(server, { grace: { milliseconds: 0 } });
    during = async () => {
        const [reserved] = await storage.database.select().from(settlement);
        await controller.reconcile(reserved!.id, reconciliation(AbortSignal.timeout(5000)));
    };
    const version = {
        spaceId,
        parentId: created.id,
        title: "Draft",
        requestId: RequestId.create(),
    };
    await expect(server.call(versions, "create", version, context)).rejects.toMatchObject({
        code: "SERVICE_UNAVAILABLE",
        message: "task-version.create outlasted its settlement grace",
    });

    // cancel it once by key
    expect([
        external,
        await storage.database.select().from(settlement),
        await storage.database.select().from(taskVersion.table),
    ]).toEqual([["copy keyed", "cancel by key"], [], []]);
});

test("predict custom methods only where their handler is, and never a scope's suspension", () => {
    // predict only methods with a handler on the client
    const unhandled = method({ permission: "archive" });
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
        methods: { get: method.get("read") },
    });
    const card = defineObject({
        name: "card",
        plural: "cards",
        scope: room,
        declarable: { schema: schema.object({}) },
        fields: {},
        permissions: ["read", "write"],
        detachable: { by: "write" },
        methods: { get: method.get("read") },
    });
    expect([Object.keys(room.methods).sort(), Object.keys(card.methods).sort()]).toEqual([
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
    // hold the space's objects in a migrated database
    const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // handle archiving
    const handled = task.handle({
        archive: async ({ target, database }) => {
            const [row] = await database
                .update(task.table)
                .set({ archived: true, revision: target!.revision + 1 })
                .where(eq(task.table.id, target!.id))
                .returning();

            return row;
        },
    });

    // decide as the current user at the current assurance, through the current delegate
    let current = "user-1";
    let level = 1;
    let agent: Subject | undefined;
    const as = (id: string) => ({
        subjects: [principal.user.reference("universe", id)],
        now: Date.now(),
        attributes: {},
        assurance: { level, authenticatedAt: Date.now() },
    });

    // serve the objects, recording each audited change in the outbox
    const server = new ObjectServer({
        objects: { task: handled, comment, taskVersion, folder },
        database,
        context: () => {
            const direct = as(current);

            return agent
                ? {
                      ...direct,
                      subject: direct.subjects[0],
                      delegates: [{ subject: agent, authority: "lent" }],
                  }
                : direct;
        },
        journal: new Journal(request, testJournalKey),
        audit: AuditRecorder.service(new AuditOutbox(database), {
            package: task.package,
            service: "test",
        }),
    });
    const observed = new Bookmark();
    const context = {
        scope: spaceId,
        requireCaller: () => ({ id: current }),
        bookmark: new Bookmark(),
        observed,
    } as unknown as ServiceContext;

    return {
        server,
        database,
        /** Read the callers' audited actions. */
        events: () => auditedActions(database, "caller"),
        /** Read the system's audited actions. */
        purges: () => auditedActions(database, "system"),
        observed,
        as,
        /** Execute a task method as the current caller. */
        execute: (name: string, input: Record<string, unknown>) =>
            server.call(handled, name, { spaceId, ...input }, context),
        /** Act as a user. */
        act: (user: string) => {
            current = user;
        },
        /** Authenticate at an assurance level. */
        step: (assurance: number) => {
            level = assurance;
        },
        /** Act through a delegate, or directly. */
        delegate: (delegate: Subject | undefined) => {
            agent = delegate;
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
    const storage = await TestDatabase.create(TEST_DIALECTS.at(-1)!, objectDatabase, {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // copy each version under a key of its title, failing the first attempt after the copy
    const keys: string[] = [];
    let isFailing = true;
    const versions = taskVersion.handle({
        create: {
            key: (call) => `copy-${String(call.input.title)}`,
            prepare: async (call) => {
                keys.push(call.key!);
                if (isFailing) {
                    isFailing = false;
                    throw new ServiceError("UNAVAILABLE", { message: "copy failed" });
                }

                return call.key;
            },
            settle: async () => {},
        },
    });
    const server = new ObjectServer({
        objects: { task, comment, taskVersion: versions, folder },
        database: storage.database,
        context: () => ({
            subjects: [principal.user.reference("universe", "user-1")],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(request, testJournalKey),
        audit: AuditRecorder.service(new AuditOutbox(storage.database), {
            package: task.package,
            service: "test",
        }),
    });
    const context = {
        scope: spaceId,
        requireCaller: () => ({ id: "user-1" }),
        bookmark: new Bookmark(),
        observed: new Bookmark(),
    } as unknown as ServiceContext;
    const created = (await server.call(
        task,
        "create",
        { spaceId, requestId: RequestId.create(), title: "Plan" },
        context,
    )) as { id: string };

    // retry the failed creation under the same request, carrying the same key both times
    const version = {
        spaceId,
        parentId: created.id,
        title: "Draft",
        requestId: RequestId.create(),
    };
    await expect(server.call(versions, "create", version, context)).rejects.toMatchObject({
        code: "UNAVAILABLE",
    });
    await server.call(versions, "create", version, context);
    expect(keys).toEqual(["copy-Draft", "copy-Draft"]);
});
