import { AuditOutbox } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { unique } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier, schema } from "@destack/schema";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import { Bookmark } from "@destack/service/bookmark";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { request } from "./schema.ts";
import { openSpace, space } from "./fixture/space.ts";
import { testJournalKey } from "@destack/service/test";

/** The public space containing the handles. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** A private space nobody reads. */
const privateId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** The authentication creating a handle asks for: several factors within ten minutes. */
const RECENT = { assurance: 2, maxAge: 10 * 60 * 1000 };

/** Names unique within a space, created only by their named user after strong authentication. */
const handle = defineObject({
    name: "handle",
    plural: "handles",
    scope: space,
    fields: {
        holder: field.reference(principal.user),
        name: field.string(schema.string().min(1)),
    },
    constraints: (entry) => [unique("handle_name").on(entry.scope, entry.name)],
    permissions: { read: relation("holder"), create: relation("holder") },
    elevated: { create: RECENT },
    methods: { get: method.get("read"), create: method.create("create") },
});

test.each(TEST_DIALECTS)(
    "report a create's conflicts only to callers the create permission admits, inside spaces they read, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [...handle.tables, request], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        await openSpace(storage.database, privateId, "private");

        // serve the handles to the user each call names
        let current = "alice";
        let level = 2;
        const server = new ObjectServer({
            objects: { handle },
            database: storage.database,
            context: () => ({
                subjects: [principal.user.reference("universe", current)],
                now: Date.now(),
                attributes: {},
                assurance: { level, authenticatedAt: Date.now() },
            }),
            journal: new Journal(request, testJournalKey),
            audit: AuditRecorder.service(new AuditOutbox(storage.database), {
                package: handle.package,
                service: "test",
            }),
        });
        const create = (user: string, input: Record<string, unknown>, scope = spaceId) => {
            current = user;
            const context = {
                scope,
                requireCaller: () => ({ id: user }),
                bookmark: new Bookmark(),
                observed: new Bookmark(),
            } as unknown as ServiceContext;

            return server.call(
                handle,
                "create",
                { spaceId: scope, requestId: RequestId.create(), ...input },
                context,
            ) as Promise<{ id: string }>;
        };

        // take a name and an identifier as alice
        const taken = await create("alice", { holder: "alice", name: "notes" });

        // tell bob of the taken name and identifier only for a handle he may create
        await expect(create("bob", { holder: "bob", name: "notes" })).rejects.toMatchObject({
            code: "DUPLICATE",
            message: "a record with the same unique key exists",
        });
        await expect(
            create("bob", { holder: "bob", name: "tasks", id: taken.id }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "handle identifier is taken" });

        // find nothing for a handle bob may not create, taken or free
        const refusals: unknown[] = [];
        for (const input of [
            { holder: "alice", name: "notes" },
            { holder: "alice", name: "tasks", id: taken.id },
            { holder: "alice", name: "tasks" },
        ]) {
            refusals.push(
                await create("bob", input).then(
                    () => undefined,
                    (error: { code: string; message: string }) => [error.code, error.message],
                ),
            );
        }
        expect(refusals).toEqual([
            ["NOT_FOUND", "handle not found"],
            ["NOT_FOUND", "handle not found"],
            ["NOT_FOUND", "handle not found"],
        ]);

        // challenge bob before a conflict, and find nothing for alice's handle
        level = 1;
        await expect(create("bob", { holder: "bob", name: "notes" })).rejects.toMatchObject({
            code: "INSUFFICIENT_AUTHENTICATION",
            stepUp: RECENT,
        });
        await expect(create("bob", { holder: "alice", name: "notes" })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "handle not found",
        });
        level = 2;

        // find nothing inside a space the caller may not read, not even its own handle
        await expect(
            create("alice", { holder: "alice", name: "notes" }, privateId),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no scope ${privateId}` });
    },
);

/** Teams any caller creates and owns, updated only by owners. */
const team = defineObject({
    name: "team",
    plural: "teams",
    scope: space,
    fields: { name: field.string(schema.string().min(1)) },
    relations: { owner: { subjects: [principal.user], grantedBy: "own" } },
    permissions: { read: relation("owner"), update: relation("owner"), own: relation("owner") },
    methods: {
        get: method.get("read"),
        create: method.create("update", { fields: ["name"], creator: "owner" }),
    },
});

test("create an object the caller holds the permission on only as the creator its creation relates", async () => {
    const storage = await TestDatabase.create("sqlite", [request, ...team.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const server = new ObjectServer({
        objects: { team },
        database: storage.database,
        context: () => ({
            subjects: [principal.user.reference("universe", "alice")],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(request, testJournalKey),
        audit: AuditRecorder.service(new AuditOutbox(storage.database), {
            package: team.package,
            service: "test",
        }),
    });
    const context = {
        scope: spaceId,
        requireCaller: () => ({ id: "alice" }),
        bookmark: new Bookmark(),
        observed: new Bookmark(),
    } as unknown as ServiceContext;

    // create a team as its owner, then read it as that owner
    const created = (await server.call(
        team,
        "create",
        { spaceId, requestId: RequestId.create(), name: "Core" },
        context,
    )) as { id: string };
    const read = (await server.call(team, "get", { spaceId, id: created.id }, context)) as {
        name: string;
    };
    expect(read.name).toBe("Core");
});
