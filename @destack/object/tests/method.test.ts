import type { CallableName } from "../src/index.ts";
import { ObjectReference, Subject, RequestId } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { reconciliation, TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { CallError } from "@destack/object";
import { type Dialect, eq } from "@destack/db";
import { nth, present, schema, toJsonSchema, type Identifier } from "@destack/schema";
import { AuditTarget, defineObject, field, method, settlement, suspendable } from "../src/index.ts";

import { v7 } from "uuid";
import { ObjectStore, RecoverableController, Settlement } from "../src/server/index.ts";
import { describeAuditAction } from "../src/inspect/index.ts";
import { comment, folder, objectDatabase, task, taskCopy, taskVersion } from "./schema.ts";
import {
    AccessContext,
    accessInvitation,
    Caller,
    type GrantPermission,
    Invitation,
    INVITATION_LIFETIME_MILLISECONDS,
    none,
    principal,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { openSpace, spacePrincipal } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext, users } from "./fixture/user.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";

/** The space containing the tasks. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The grant a delegate acts under for the current user. */
const grantId = schema.identifier("grant").parse("grant-01996ab0-0000-7000-8000-000000000002");

/** The routes of derived procedures, by method name. */
const PROCEDURES = schema.record(
    schema.string(),
    schema.looseObject({
        "~orpc": schema.looseObject({
            route: schema.looseObject({ method: schema.string(), path: schema.string() }),
        }),
    }),
);

test("derive routes and describe each method with its permission, input and audit action", () => {
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
        archiveAll: "POST /spaces/{spaceId}/tasks/archive-all",
        export: "POST /spaces/{spaceId}/tasks/{id}/export",
        complete: "POST /spaces/{spaceId}/tasks/{id}/complete",
        reopen: "POST /spaces/{spaceId}/tasks/{id}/reopen",
        restore: "POST /spaces/{spaceId}/tasks/{id}/restore",
        purge: "POST /spaces/{spaceId}/tasks/{id}/purge",
        relationships: "GET /spaces/{spaceId}/tasks/{id}/relationships",
        grant: "POST /spaces/{spaceId}/tasks/{id}/relationships",
        revoke: "DELETE /spaces/{spaceId}/tasks/{id}/relationships/{relationshipId}",
        invitations: "GET /spaces/{spaceId}/tasks/{id}/invitations",
        invite: "POST /spaces/{spaceId}/tasks/{id}/invitations",
        accept: "POST /spaces/{spaceId}/tasks/{id}/invitations/{invitationId}/accept",
        withdraw: "DELETE /spaces/{spaceId}/tasks/{id}/invitations/{invitationId}",
        explain: "GET /spaces/{spaceId}/tasks/{id}/access",
    });

    // audit each change with its target, and reads only of read-audited types
    expect([describeAuditAction(task.audit("archive")), task.isReadAudited]).toEqual([
        {
            name: "task.archive",
            package: {
                id: task.policy.definition.packageId,
                name: "@destack/object",
                version: task.package.version,
            },
            target: toJsonSchema(AuditTarget),
            details: toJsonSchema(task.audit("archive").details),
        },
        false,
    ]);

    // take every writable field in an update, requiring only the target and request
    const update = present(task.procedures["update"], "the update procedure")["~orpc"];
    const updateInput = schema
        .looseObject({
            properties: schema.record(schema.string(), schema.unknown()),
            required: schema.array(schema.string()),
        })
        .parse(toJsonSchema(present(update.inputSchema, "the update input")));
    expect([Object.keys(updateInput.properties), updateInput.required]).toEqual([
        ["spaceId", "id", "requestId", "revision", "title", "estimate", "currentId"],
        ["spaceId", "id", "requestId"],
    ]);
});

test("route a creation other than create below its collection by name", () => {
    // declare a type created by create and by import
    const note = defineObject(
        {
            name: "note",
            plural: "notes",
            scope: "universe",
            fields: { title: field.string(schema.string().min(1)) },
            permissions: { write: none() },
        },
        (declared) => ({
            create: declared.create("write", { fields: ["title"] }),
            import: declared.create("write", { fields: ["title"] }),
        }),
    );

    // route create at the collection and import below it
    const routes = PROCEDURES.parse(note.procedures);
    expect(
        ["create", "import"].map((name) => {
            const { route } = present(routes[name], `the ${name} procedure`)["~orpc"];

            return `${route.method} ${route.path}`;
        }),
    ).toEqual(["POST /notes", "POST /notes/import"]);
});

test.each(TEST_DIALECTS)(
    "route every served object, create once per request identifier and take chosen identifiers on %s",
    async (dialect) => {
        const { store, database, execute, events, observed } = await serveTasks(dialect);

        // route the replica procedures and each served object
        expect(Object.keys(store.router())).toEqual([
            "store",
            "task",
            "comment",
            "taskVersion",
            "folder",
        ]);

        // create once per request identifier, replaying the original result
        const creation = { requestId: RequestId.create(Date.now()), title: "Write the plan" };
        const created = await execute("create", creation);
        expect(observed().watermarks.at(-1)).toEqual(await store.watermark());
        expect(await execute("create", creation)).toEqual(created);

        // take a chosen identifier and refuse a taken one
        const chosen = schema.identifier("task").parse(`task-${v7()}`);
        const drafted = { requestId: RequestId.create(Date.now()), id: chosen, title: "Draft" };
        expect((await execute("create", drafted)).id).toBe(chosen);
        await expect(
            execute("create", {
                requestId: RequestId.create(Date.now()),
                id: chosen,
                title: "Again",
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "task identifier is taken" });
        const ahead = schema
            .identifier("task")
            .parse(`task-${v7({ msecs: Date.now() + 3_600_000 })}`);
        const later = { requestId: RequestId.create(Date.now()), id: ahead, title: "Later" };
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
    "hide an object from strangers, update it at the observed revision, with sensitive fields out of rows on %s",
    async (dialect) => {
        const { database, execute, events, act } = await serveTasks(dialect);
        const created = await execute("create", {
            requestId: RequestId.create(Date.now()),
            title: "Write the plan",
        });

        // hide the task from another user, and refuse them changes without revealing it
        act(users.grace);
        expect((await execute("list", {})).items).toEqual([]);
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${created.id}`,
        });
        await expect(
            execute("update", {
                id: created.id,
                requestId: RequestId.create(Date.now()),
                revision: created.revision,
                title: "Taken",
            }),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no task ${created.id}` });
        act(users.frank);

        // update at the observed revision and reject a stale one
        const updated = await execute("update", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            revision: created.revision,
            title: "Ship the plan",
            estimate: 3,
        });
        expect(updated.estimate).toBe(3);
        expect([updated.title, updated.revision]).toEqual(["Ship the plan", 2]);
        await expect(
            execute("update", {
                id: created.id,
                requestId: RequestId.create(Date.now()),
                revision: created.revision,
                title: "Stale",
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "task revision has changed" });

        // store a sensitive field in the database, out of every row a caller reads and its schema
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
    "share an object, accept invitations and narrow agents to their grants on %s",
    async (dialect) => {
        const { clock, execute, events, act, delegate } = await serveTasks(dialect);
        const created = await execute("create", {
            requestId: RequestId.create(Date.now()),
            title: "Ship the plan",
        });

        // share the task with another user as a viewer, list it, and revoke it again
        const viewer = principal.user.reference("universe", users.grace);
        const shared = await execute("grant", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
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
                    createdAt: clock.now(),
                    expiresAt: null,
                },
            ],
            cursor: null,
        });
        act(users.grace);
        expect((await execute("get", { id: created.id })).title).toBe("Ship the plan");
        await expect(
            execute("grant", {
                id: created.id,
                requestId: RequestId.create(Date.now()),
                relation: "viewer",
                subject: { ...viewer, id: users.ivan },
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: write" });
        act(users.frank);
        await execute("revoke", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            relationshipId: shared.id,
        });
        act(users.grace);
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${created.id}`,
        });
        act(users.frank);

        // let a user without access ask for it, and apply it once the owner accepts
        act(users.heidi);
        const asked = await execute("invite", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            relationship: { relation: "viewer", subject: { ...viewer, id: users.heidi } },
            purpose: "follow the plan",
        });
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${created.id}`,
        });
        await expect(
            execute("accept", {
                id: created.id,
                requestId: RequestId.create(Date.now()),
                invitationId: asked.id,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: write" });
        act(users.frank);
        expect(await execute("invitations", { id: created.id })).toEqual({
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
                        subject: { ...viewer, id: users.heidi },
                    },
                    inviter: { ...viewer, id: users.heidi },
                    purpose: "follow the plan",
                    status: "pending",
                    createdAt: clock.now(),
                    expiresAt: clock.now() + INVITATION_LIFETIME_MILLISECONDS,
                },
            ],
            cursor: null,
        });
        await execute("accept", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            invitationId: asked.id,
        });
        act(users.heidi);
        expect((await execute("get", { id: created.id })).title).toBe("Ship the plan");
        act(users.frank);

        // challenge an agent whose grant gives too little, and allow what it gives
        const agent = principal.installation.reference(spaceId, "assistant");
        await delegate(agent, [{ ...task.permission("write"), scope: spaceId }]);
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "INSUFFICIENT_GRANT",
            message: `grant ${grantId} does not give read on task ${created.id}`,
            data: { grant: grantId, object: task.reference(spaceId, created.id) },
        });
        await delegate(agent, [{ ...task.permission("read"), scope: spaceId }]);
        expect((await execute("get", { id: created.id })).title).toBe("Ship the plan");
        await expect(
            execute("update", {
                id: created.id,
                requestId: RequestId.create(Date.now()),
                revision: 1,
                title: "Taken",
            }),
        ).rejects.toMatchObject({
            code: "INSUFFICIENT_GRANT",
            message: `grant ${grantId} does not give write on task ${created.id}`,
        });
        await delegate(undefined, []);

        // audit each change once
        expect(await events()).toEqual([
            "task.create",
            "task.grant",
            "task.revoke",
            "task.invite",
            "task.accept",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "hide guarded fields from readers without their permission, and run each transition once on %s",
    async (dialect) => {
        const { execute, events, act } = await serveTasks(dialect);
        const created = await execute("create", {
            requestId: RequestId.create(Date.now()),
            title: "Write the plan",
        });
        await execute("update", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            revision: created.revision,
            title: "Ship the plan",
            estimate: 3,
        });
        const viewer = principal.user.reference("universe", users.heidi);
        await execute("grant", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            relation: "viewer",
            subject: viewer,
        });

        // hide the guarded estimate and refuse setting it
        act(users.heidi);
        const viewed = await execute("get", { id: created.id });
        expect([viewed.title, "estimate" in viewed]).toEqual(["Ship the plan", false]);
        expect((await execute("list", {})).items.map((item) => "estimate" in item)).toEqual([
            false,
        ]);
        act(users.frank);
        await execute("grant", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            relation: "editor",
            subject: { ...viewer, id: users.judy },
        });
        act(users.judy);
        const edited = await execute("update", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            revision: 2,
            title: "Ship the plan",
        });
        expect([edited.revision, "estimate" in edited]).toEqual([3, false]);
        await expect(
            execute("update", {
                id: created.id,
                requestId: RequestId.create(Date.now()),
                revision: 3,
                title: "Ship the plan",
                estimate: 8,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "field estimate is not writable" });
        act(users.frank);

        // complete the task once and refuse a second and a guarded completion
        const completed = await execute("complete", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
        });
        expect(completed.status).toBe("done");
        await expect(
            execute("complete", { id: created.id, requestId: RequestId.create(Date.now()) }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "task cannot complete while status is done",
        });
        await execute("reopen", { id: created.id, requestId: RequestId.create(Date.now()) });

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
        const { store, database, execute, events, purges } = await serveTasks(dialect);
        const created = await execute("create", {
            requestId: RequestId.create(Date.now()),
            title: "Ship the plan",
        });

        // refuse standard changes to a managed task
        const archived = await execute("archive", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
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
                requestId: RequestId.create(Date.now()),
                revision: archived.revision,
            }),
        ).rejects.toMatchObject({
            code: "MANAGED",
            message: "task is managed by its stack; detach it before changing it",
        });

        // delete once detached: hidden from lists, listed in the trash, restorable and purged at last
        await database
            .update(task.table)
            .set({ detachedAt: Date.now() })
            .where(eq(task.table.id, created.id));
        await execute("delete", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            revision: archived.revision,
        });
        const titles = async (deleted: "exclude" | "only") =>
            (await execute("list", { deleted })).items.map((item) => item.title);
        expect([await titles("exclude"), await titles("only")]).toEqual([[], ["Ship the plan"]]);
        await execute("restore", { id: created.id, requestId: RequestId.create(Date.now()) });
        expect(await titles("exclude")).toEqual(["Ship the plan"]);
        const restored = await execute("get", {
            id: created.id,
        });
        await execute("delete", {
            id: created.id,
            requestId: RequestId.create(Date.now()),
            revision: restored.revision,
        });
        await execute("purge", { id: created.id, requestId: RequestId.create(Date.now()) });
        await expect(execute("get", { id: created.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no task ${created.id}`,
        });

        // purge the trash once the recovery window passes
        const stale = await execute("create", {
            requestId: RequestId.create(Date.now()),
            title: "Old",
        });
        await execute("delete", {
            id: stale.id,
            requestId: RequestId.create(Date.now()),
            revision: stale.revision,
        });
        expect(await RecoverableController.purge(store, Date.now() + 60_000)).toBe(1);

        // purge more trash than one batch takes, across several transactions
        const trashed = Array.from({ length: 100 }, () => ({
            ...stale,
            id: schema.identifier("task").parse(`task-${v7()}`),
            deletedAt: Date.now(),
        }));
        await database.insert(task.table).values(trashed);
        expect(await RecoverableController.purge(store, Date.now() + 60_000)).toBe(100);

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

test("page an object's invitations a full hundred at a time", async () => {
    // create 101 pending invitations to view a task, in creation order
    const { database, execute } = await serveTasks("sqlite");
    const created = await execute("create", {
        requestId: RequestId.create(Date.now()),
        title: "Ship the plan",
    });
    const object = task.reference(spaceId, created.id);
    const invited = Array.from({ length: 101 }, (_, index) => `user-${100 + index}`);
    await database.insert(accessInvitation).values(
        invited.map((id, index) =>
            Invitation.encode(
                {
                    id: `invitation-${v7({ msecs: 1 + index })}`,
                    relationship: {
                        object,
                        relation: "viewer",
                        subject: principal.user.reference("universe", id),
                        expiresAt: null,
                    },
                    inviter: principal.user.reference("universe", users.frank),
                    status: "pending",
                    createdAt: 1,
                    expiresAt: Date.now() + 60_000,
                },
                spaceId,
            ),
        ),
    );

    // read a page of 100 with a cursor, then the last one
    const Listed = schema.object({
        items: schema.array(
            schema.looseObject({
                relationship: schema.looseObject({
                    subject: schema.looseObject({ id: schema.string() }),
                }),
            }),
        ),
        cursor: schema.string().nullable(),
    });
    const first = Listed.parse(await execute("invitations", { id: created.id, limit: 100 }));
    const last = Listed.parse(
        await execute("invitations", {
            id: created.id,
            limit: 100,
            cursor: present(first.cursor, "the next page's cursor"),
        }),
    );

    expect({
        first: first.items.map((item) => item.relationship.subject.id),
        last: last.items.map((item) => item.relationship.subject.id),
        cursor: last.cursor,
    }).toEqual({
        first: invited.slice(0, 100),
        last: invited.slice(100),
        cursor: null,
    });
});

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
    const handled = task.implement({
        methods: {
            export: {
                prepare: async (context) => {
                    if (context.target.title === "Broken") {
                        throw new Error("reservation failed");
                    }
                    external.push(`reserve ${context.target.title}`);

                    return `reservation-${context.target.id}`;
                },
                handle: async (context) => {
                    if (isFailing) {
                        throw new Error("archive failed");
                    }

                    return context.update({
                        archived: true,
                        title: `${context.target.title} (${context.prepared})`,
                    });
                },
                settle: async (_call, settled) => {
                    external.push(
                        settled.status === "committed"
                            ? `confirm ${settled.prepared}`
                            : `release ${String(settled.prepared)}`,
                    );
                },
            },
        },
    });
    const clock = new VirtualClock(Date.now());
    const store = new ObjectStore({
        chain: async () => undefined,
        clock,
        principal: spacePrincipal(spaceId),
        servers: serversOf([handled, comment, taskVersion, folder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: {
            package: task.package,
        },
    });
    let context = userContext(users.frank, spaceId);
    const execute = <Name extends CallableName<typeof handled>>(
        name: Name,
        input: NoInfer<RequestInput<typeof handled, Name>>,
    ) => request(store, handled, name, spaceId, input, context);
    const created = await execute("create", {
        requestId: RequestId.create(Date.now()),
        title: "Plan",
    });
    const reservation = `reservation-${created.id}`;

    // refuse a caller without the permission before any external work
    context = userContext(users.grace, spaceId);
    await expect(
        execute("export", { requestId: RequestId.create(Date.now()), id: created.id }),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no task ${created.id}` });
    expect(external).toEqual([]);

    // release the prepared work and leave the task unchanged when the transaction fails
    context = userContext(users.frank, spaceId);
    isFailing = true;
    await expect(
        execute("export", { requestId: RequestId.create(Date.now()), id: created.id }),
    ).rejects.toThrow(new Error("archive failed"));
    expect(external).toEqual(["reserve Plan", `release ${reservation}`]);

    // confirm after commit and replay without repeating the work
    isFailing = false;
    external.length = 0;
    const exported = { requestId: RequestId.create(Date.now()), id: created.id };
    const archived = await execute("export", exported);
    const author = principal.user.reference("universe", users.frank);
    expect(archived).toEqual({
        id: created.id,
        createdAt: clock.now(),
        ...ObjectReference.columns("createdBy", author),
        updatedAt: clock.now(),
        ...ObjectReference.columns("updatedBy", author),
        ...ObjectReference.columns("deletedBy", undefined),
        revision: 2,
        tags: {},
        scope: spaceId,
        deletedAt: null,
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
        ownerId: users.frank,
        title: `Plan (${reservation})`,
        archived: true,
        estimate: null,
        ...ObjectReference.columns("origin", undefined),
        currentId: null,
        status: "open",
    });
    expect(await execute("export", exported)).toEqual(archived);
    expect(external).toEqual(["reserve Plan", `confirm ${reservation}`]);

    // release the work prepared for earlier calls and the failed preparation's own once a later call's preparation fails
    const other = await execute("create", {
        requestId: RequestId.create(Date.now()),
        title: "Other",
    });
    const broken = await execute("create", {
        requestId: RequestId.create(Date.now()),
        title: "Broken",
    });
    external.length = 0;
    await expect(
        store.mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [
                handled.call("export", { spaceId, id: other.id }),
                handled.call("export", { spaceId, id: broken.id }),
            ],
        }),
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
        store.mutate(userContext(users.frank, undefined).access(), {
            id: RequestId.create(Date.now()),
            calls: [
                handled.call("export", { spaceId, id: other.id }),
                handled.call("export", { spaceId: elsewhere, id: other.id }),
            ],
        }),
    ).rejects.toMatchObject({
        code: "BAD_REQUEST",
        message: "a mutation's calls must act in one scope",
    });
    expect(external).toEqual([]);
});

test("run a prepare and a settle phase's calls as the store in requests of their own, committed apart from the mutation", async () => {
    const storage = await TestDatabase.create("sqlite", objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // rename a log task as the store while preparing and settling an export, then fail the export
    const log: { id?: string } = {};
    const seen: string[] = [];
    const rename = (title: string) =>
        task.call("update", { spaceId, id: present(log.id, "the log task"), title });
    const handled = task.implement({
        methods: {
            export: {
                prepare: async (context) => {
                    await context.mutate(context.self, [rename("prepared")]);

                    return "reserved";
                },
                handle: async (context) => {
                    const id = task.identifier(present(log.id, "the log task"));
                    const read = await context.read(task).findFirst({ where: { id } });
                    seen.push(present(read, "the log task").title);
                    throw new Error("export failed");
                },
                settle: async (context) => {
                    await context.mutate(context.self, [rename("settled")]);
                },
            },
        },
    });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([handled, comment, taskVersion, folder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: task.package },
    });
    const context = userContext(users.frank, spaceId);
    const execute = <Name extends CallableName<typeof handled>>(
        name: Name,
        input: NoInfer<RequestInput<typeof handled, Name>>,
    ) => request(store, handled, name, spaceId, input, context);
    const plan = await execute("create", {
        requestId: RequestId.create(Date.now()),
        title: "Plan",
    });
    const created = await execute("create", {
        requestId: RequestId.create(Date.now()),
        title: "Log",
    });
    log.id = created.id;

    // see the prepared rename inside the failing mutation, and the settled one after it
    await expect(
        execute("export", { requestId: RequestId.create(Date.now()), id: plan.id }),
    ).rejects.toThrow("export failed");
    const after = await execute("get", { id: created.id });

    expect({ seen, title: after.title }).toEqual({ seen: ["prepared"], title: "settled" });
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
    const copies = taskCopy.implement({
        methods: {
            create: {
                prepare: async (context) => {
                    external.push(`copy ${context.input["title"]}`);

                    return `copy-${present(context.id, "the version's identifier")}`;
                },
                settle: async (_call, settled) => {
                    // fail the first confirmation, then confirm or release
                    if (settled.status === "committed" && !isSettling) {
                        isSettling = true;
                        throw new Error("confirmation failed");
                    }
                    external.push(
                        settled.status === "committed"
                            ? `confirm ${settled.prepared}`
                            : `release ${String(settled.prepared)}`,
                    );
                },
            },
        },
    });
    const reported: unknown[] = [];
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([task, comment, taskVersion, copies, folder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: {
            package: task.package,
        },
        report: (error) => reported.push(error),
    });
    let context = userContext(users.frank, spaceId);
    const created = await store
        .mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [task.call("create", { spaceId, title: "Plan" })],
        })
        .then(([written]) => written);

    // refuse a caller who may not write the task before any external work
    context = userContext(users.grace, spaceId);
    const version = { spaceId, parentId: created.id, title: "Draft" };
    await expect(
        store
            .mutate(context.access(), {
                id: RequestId.create(Date.now()),
                calls: [copies.call("create", { ...version })],
            })
            .then(([written]) => written),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no task ${created.id}` });
    expect(external).toEqual([]);

    // commit and retain the settlement despite a failed confirmation
    context = userContext(users.frank, spaceId);
    const drafted = await store
        .mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [copies.call("create", { ...version })],
        })
        .then(([written]) => written);
    const settled = await storage.database.select().from(settlement);
    expect([external, settled.map((row) => [row.method, row.target]), reported]).toEqual([
        ["copy Draft"],
        [["create", drafted.id]],
        [new Error("confirmation failed")],
    ]);

    // confirm it through the controller
    const controller = Settlement.controller(store, { grace: { milliseconds: 0 } });
    expect(
        await controller.reconcile(nth(settled, 0).id, reconciliation(AbortSignal.timeout(5000))),
    ).toBeUndefined();
    expect([external, await storage.database.select().from(settlement)]).toEqual([
        ["copy Draft", `confirm copy-${drafted.id}`],
        [],
    ]);
});

test("name a system creation's prepared work by the fresh identifier it creates the record under", async () => {
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        objectDatabase,
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // key the external copy by the identifier the system creation takes
    const external: string[] = [];
    const copies = taskCopy.implement({
        methods: {
            create: {
                prepare: async (context) => {
                    const copy = `copy-${present(context.id, "the version's identifier")}`;
                    external.push(copy);

                    return copy;
                },
            },
        },
    });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([task, comment, taskVersion, copies, folder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: task.package },
    });
    const created = await store
        .mutate(userContext(users.frank, spaceId).access(), {
            id: RequestId.create(Date.now()),
            calls: [task.call("create", { spaceId, title: "Plan" })],
        })
        .then(([written]) => written);
    const [drafted] = await store.mutate(
        AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()),
        {
            id: RequestId.create(Date.now()),
            calls: [
                copies.call("create", {
                    ...copies.in(spaceId),
                    parentId: created.id,
                    title: "Draft",
                }),
            ],
        },
    );

    expect(external).toEqual([`copy-${present(drafted, "the copy").id}`]);
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

    // store external copies by call key, pausing mid-copy
    const external: string[] = [];
    let during: () => Promise<void> = idle;
    const copies = taskCopy.implement({
        methods: {
            create: {
                prepare: async (context) => {
                    external.push(
                        `copy ${context.idempotencyKey === undefined ? "unkeyed" : "keyed"}`,
                    );
                    await during();

                    return present(context.idempotencyKey, "the copy's key");
                },
                settle: async (_call, settled) => {
                    external.push(
                        settled.status === "committed"
                            ? "confirm prepared"
                            : `cancel ${settled.prepared === undefined ? "by key" : "prepared"}`,
                    );
                },
            },
        },
    });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([task, comment, taskVersion, copies, folder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: {
            package: task.package,
        },
    });
    const context = userContext(users.frank, spaceId);
    const created = await store
        .mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [task.call("create", { spaceId, title: "Plan" })],
        })
        .then(([written]) => written);

    // cancel the reserved settlement mid-copy
    const controller = Settlement.controller(store, { grace: { milliseconds: 0 } });
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
    };
    await expect(
        store
            .mutate(context.access(), {
                id: RequestId.create(Date.now()),
                calls: [copies.call("create", version)],
            })
            .then(([written]) => written),
    ).rejects.toMatchObject({
        code: "SERVICE_UNAVAILABLE",
        message: "task-copy.create exceeded its settlement grace period",
    });

    // cancel it once by key
    expect([
        external,
        await storage.database.select().from(settlement),
        await storage.database.select().from(taskCopy.table),
    ]).toEqual([["copy keyed", "cancel by key"], [], []]);
});

test.each(TEST_DIALECTS)(
    "run a scoped mutation over its scope as a holder of the scope's permission, and refuse a caller without it on %s",
    async (dialect) => {
        // create two tasks as their owner and archive one
        const { store, execute } = await serveTasks(dialect);
        const first = await execute("create", {
            requestId: RequestId.create(Date.now()),
            title: "First",
        });
        await execute("create", { requestId: RequestId.create(Date.now()), title: "Second" });
        await execute("archive", { requestId: RequestId.create(Date.now()), id: first.id });

        // refuse the owner of the tasks, who does not own the space
        const refused = await execute("archiveAll", {
            requestId: RequestId.create(Date.now()),
        }).catch((error: unknown) => error);

        // archive the rest as the space, which owns it
        const [archived] = await store.mutate(
            AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()),
            {
                id: RequestId.create(Date.now()),
                calls: [task.call("archiveAll", task.in(spaceId))],
            },
        );
        const left = await execute("list", { where: { archived: false } });
        expect([refused, archived, left.items]).toEqual([
            new CallError("FORBIDDEN", { defined: true, message: "permission denied: tidy" }),
            { archived: 1 },
            [],
        ]);
    },
);

test("create the same identifiers and draw the same numbers from the same request on every server, apart for each draw", async () => {
    // create two tasks under one request and draw under another, on each of two servers
    const [creation, drawing] = [RequestId.create(Date.now()), RequestId.create(Date.now())];
    const run = async () => {
        const { store } = await serveTasks("sqlite");
        const caller = AccessContext.of(
            Caller.of(principal.user.reference("universe", users.frank)),
            Date.now(),
        );
        const created = await store.mutate(caller, {
            id: creation,
            calls: [
                task.call("create", { ...task.in(spaceId), title: "First" }),
                task.call("create", { ...task.in(spaceId), title: "Second" }),
            ],
        });
        const [drawn] = await store.mutate(
            AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()),
            {
                id: drawing,
                calls: [folder.call("draw", folder.in(spaceId))],
            },
        );

        return [
            ...created.map((row) => schema.looseObject({ id: schema.string() }).parse(row).id),
            drawn,
        ];
    };
    const first = await run();
    const second = await run();

    // repeat every identifier and draw, each apart from the others
    const [one, two, drawn] = first;
    const { id, draws } = schema
        .object({ id: schema.string(), draws: schema.array(schema.number()) })
        .parse(drawn);
    expect({
        isRepeated: JSON.stringify(first) === JSON.stringify(second),
        distinct: new Set([one, two, id]).size + new Set(draws).size,
        isUnit: draws.every((draw) => draw >= 0 && draw < 1),
    }).toEqual({ isRepeated: true, distinct: 5, isUnit: true });
});

test("share an implemented type's procedures with its declaration, and build a changed type's afresh", () => {
    // implement the task type twice and change it once
    const implemented = [task.implement({}), task.implement({})];
    const changed = task.with({ methods: { ...task.methods } });

    // reuse the declared procedure in each implementation, and build the changed type's anew
    const declared = task.procedure("get");
    expect([
        ...implemented.map((type) => type.procedure("get") === declared),
        changed.procedure("get") === declared,
    ]).toEqual([true, true, false]);
});

test("predict custom methods only where their handler is, and never a scope's suspension", () => {
    // predict only methods with a handler on the client
    const unhandled = method.mutation({ permission: "archive" });
    const handled = unhandled.handle(async (context) => context.target);
    expect([
        unhandled.isPredicted,
        handled.isPredicted,
        suspendable.suspend("update").isPredicted,
        suspendable.resume("update").isPredicted,
    ]).toEqual([false, true, false, false]);
});

test("add suspending and handing over through their traits", () => {
    // derive suspend and resume on a scope type, and detach and adopt on a declarable type
    const room = defineObject({
        name: "room",
        plural: "rooms",
        scope: "universe",
        isScope: true,
        fields: {},
        permissions: ["read", "update"],
        suspendable: { by: "update" },
    });
    const card = defineObject({
        name: "card",
        plural: "cards",
        scope: room,
        declarable: { schema: schema.object({}) },
        fields: {},
        permissions: ["read", "write"],
        adoptable: { by: "write" },
    });
    expect([Object.keys(room.methods).toSorted(), Object.keys(card.methods).toSorted()]).toEqual([
        ["get", "list", "resume", "suspend"],
        ["adopt", "apply", "create", "delete", "detach", "get", "list", "update"],
    ]);
});

/** Serve tasks, comments, versions and folders in a space to a scenario's caller. */
async function serveTasks(dialect: Dialect) {
    // store the space's objects in a migrated database
    const database = await openTaskDatabase(dialect);

    // handle archiving
    const handled = handleArchive();

    // decide as the current user, through the current delegate
    let current = users.frank;
    let agent: Subject | undefined;
    const sign = () =>
        userContext(current, spaceId, {
            assurance: { level: 1, authenticatedAt: Date.now() },
            ...(agent === undefined
                ? {}
                : {
                      actors: [agent],
                      grants: [{ id: grantId, scope: current }],
                  }),
        });
    let context = sign();

    // serve the objects at a fixed time, recording each audited change in the outbox
    const clock = new VirtualClock(Date.now());
    const store = new ObjectStore({
        chain: async () => undefined,
        clock,
        principal: spacePrincipal(spaceId),
        servers: serversOf([handled, comment, taskVersion, handleDraw()]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: {
            package: task.package,
        },
    });

    return {
        store,
        database,
        clock,
        /** Read the callers' audited actions. */
        events: () => auditedActions(database, "caller"),
        /** Read the system's audited actions. */
        purges: () => auditedActions(database, "component"),
        /** Read the watermarks the current caller's requests reached. */
        observed: () => context.observed,
        /** Execute a task method as the current caller. */
        execute: <Name extends CallableName<typeof handled>>(
            name: Name,
            input: NoInfer<RequestInput<typeof handled, Name>>,
        ) => request(store, handled, name, spaceId, input, context),
        /** Act as a user. */
        act: (user: Identifier<"user">) => {
            current = user;
            context = sign();
        },
        /** Act through a delegate under a stored grant of some permissions, or directly. */
        delegate: async (delegate: Subject | undefined, permissions: GrantPermission[]) => {
            // store the current user's grant to the delegate
            agent = delegate;
            if (delegate !== undefined) {
                const delegator = principal.user.reference("universe", current);
                await new AccessFixture(database).replicateGrant(
                    delegator,
                    delegate,
                    permissions,
                    Date.now(),
                    grantId,
                );
            }
            context = sign();
        },
    };
}

test("carry a key of the call's place in its request, the same for every retry of the request", async () => {
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        objectDatabase,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // copy each version under its call's key, failing the first attempt after the copy
    const keys: string[] = [];
    let isFailing = true;
    const copies = taskCopy.implement({
        methods: {
            create: {
                prepare: async (context) => {
                    const key = present(context.idempotencyKey, "the call's key");
                    keys.push(key);
                    if (isFailing) {
                        isFailing = false;
                        throw new CallError("SERVICE_UNAVAILABLE", { message: "copy failed" });
                    }

                    return key;
                },
                settle: async () => {},
            },
        },
    });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([task, comment, taskVersion, copies, folder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: {
            package: task.package,
        },
    });
    const context = userContext(users.frank, spaceId);
    const created = await store
        .mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [task.call("create", { spaceId, title: "Plan" })],
        })
        .then(([written]) => written);

    // retry the failed creation under the same request with the same key both times
    const version = {
        spaceId,
        parentId: created.id,
        title: "Draft",
    };
    const requestId = RequestId.create(Date.now());
    await expect(
        store
            .mutate(context.access(), {
                id: requestId,
                calls: [copies.call("create", version)],
            })
            .then(([written]) => written),
    ).rejects.toMatchObject({
        code: "SERVICE_UNAVAILABLE",
    });
    await store
        .mutate(context.access(), {
            id: requestId,
            calls: [copies.call("create", version)],
        })
        .then(([written]) => written);
    expect(keys).toEqual([`${requestId}/0`, `${requestId}/0`]);
});

test("settle each call's prepared work once under its own key in its request", async () => {
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        objectDatabase,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // copy under each call's key, confirming each copy
    const confirmed: unknown[] = [];
    const copies = taskCopy.implement({
        methods: {
            create: {
                prepare: async (context) => present(context.idempotencyKey, "the call's key"),
                settle: async (_call, settled) => {
                    confirmed.push(settled.prepared);
                },
            },
        },
    });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([task, comment, taskVersion, copies, folder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: {
            package: task.package,
        },
    });
    const context = userContext(users.frank, spaceId);
    const created = await store
        .mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [task.call("create", { spaceId, title: "Plan" })],
        })
        .then(([written]) => written);

    // create two copies of one title in one mutation, confirming each under its own key leaving no settlement
    const copy = { spaceId, parentId: created.id, title: "Draft" };
    const requestId = RequestId.create(Date.now());
    await store.mutate(context.access(), {
        id: requestId,
        calls: [copies.call("create", copy), copies.call("create", copy)],
    });
    expect([confirmed, await storage.database.select().from(settlement)]).toEqual([
        [`${requestId}/0`, `${requestId}/1`],
        [],
    ]);
});

/** Wait for nothing, until a test pauses a call in its place. */
async function idle(): Promise<void> {}

/** Open a migrated database holding the scenario's space until the test finishes. */
async function openTaskDatabase(dialect: Dialect) {
    const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    return storage.database;
}

/** Handle the task's archive method by marking its row archived. */
/** Handle drawing an identifier and two numbers from a call's request. */
function handleDraw() {
    return folder.implement({
        methods: {
            draw: async (call) => ({
                id: await call.generateId(task),
                draws: [await call.random(), await call.random()],
            }),
        },
    });
}

/** Implement the task's archive method by setting its archived field. */
function handleArchive() {
    return task.implement({
        methods: {
            archive: (call) => call.update({ archived: true }),
            archiveAll: async (call) => {
                // archive every open task of the scope in the call's transaction
                const scope = schema.identifier("space").parse(call.scope);
                const open = await call.read(task).findMany({ where: { scope, archived: false } });
                await call.mutate(
                    call.caller,
                    open.map((row) => task.call("archive", task.target(row))),
                );

                return { archived: open.length };
            },
        },
    });
}

test("refuse a handler naming a key that is no phase, such as a misspelled handle", () => {
    // pass phases the compiler cannot see, as an untyped caller would
    const phases = { expand: async () => [], handler: async () => ({}) };

    expect(() => {
        Reflect.apply(task.implement.bind(task), undefined, [{ methods: { archive: phases } }]);
    }).toThrow(new TypeError("a method's handler has no phase handler"));
});
