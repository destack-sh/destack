import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { unique } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";
import { userContext } from "./fixture/user.ts";

/** The public space containing the handles. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** A private space nobody reads. */
const privateId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

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
    methods: (method) => ({ get: method.get("read"), create: method.create("create") }),
});

test.each(TEST_DIALECTS)(
    "report a create's conflicts only to callers the create permission admits, inside spaces they read, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [...handle.tables, journal], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        await openSpace(storage.database, privateId, "private");

        // serve the handles to the user each call names
        let level = 2;
        const server = new ObjectServer({
            objects: { handle },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: handle.package,
                service: "test",
            },
        });
        const create = (user: string, input: Record<string, unknown>, scope = spaceId) => {
            const context = userContext(user, scope, {
                assurance: { level, authenticatedAt: Date.now() },
            });

            return server.call(
                handle,
                "create",
                { spaceId: scope, requestId: RequestId.create(), ...input },
                context,
            );
        };

        // take a name and an identifier as alice
        const taken = await create("alice", { holderId: "alice", name: "notes" });

        // tell bob of the taken name and identifier only for a handle he may create
        await expect(create("bob", { holderId: "bob", name: "notes" })).rejects.toMatchObject({
            code: "DUPLICATE",
            message: "a record with the same unique key exists",
        });
        await expect(
            create("bob", { holderId: "bob", name: "tasks", id: taken.id }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "handle identifier is taken" });

        // find nothing for a handle bob may not create, taken or free
        const refusals: unknown[] = [];
        for (const input of [
            { holderId: "alice", name: "notes" },
            { holderId: "alice", name: "tasks", id: taken.id },
            { holderId: "alice", name: "tasks" },
        ]) {
            refusals.push(await refusal(create("bob", input)));
        }
        expect(refusals).toEqual([
            ["NOT_FOUND", "handle not found"],
            ["NOT_FOUND", "handle not found"],
            ["NOT_FOUND", "handle not found"],
        ]);

        // challenge bob before a conflict, and find nothing for alice's handle
        level = 1;
        await expect(create("bob", { holderId: "bob", name: "notes" })).rejects.toMatchObject({
            code: "INSUFFICIENT_AUTHENTICATION",
            stepUp: RECENT,
        });
        await expect(create("bob", { holderId: "alice", name: "notes" })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "handle not found",
        });
        level = 2;

        // find nothing inside a space the caller may not read, not even its own handle
        await expect(
            create("alice", { holderId: "alice", name: "notes" }, privateId),
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
    methods: (method) => ({
        get: method.get("read"),
        create: method.create("update", { fields: ["name"], creator: "owner" }),
    }),
});

test("create an object the caller has the permission on only as the creator its creation relates", async () => {
    const storage = await TestDatabase.create("sqlite", [journal, ...team.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const server = new ObjectServer({
        objects: { team },
        database: storage.database,
        callKey: testCallKey,
        origin: {
            package: team.package,
            service: "test",
        },
    });
    const context = userContext("alice", spaceId);

    // create a team as its owner and read it as that owner
    const created = await server.call(
        team,
        "create",
        { spaceId, requestId: RequestId.create(), name: "Core" },
        context,
    );
    const read = await server.call(team, "get", { spaceId, id: created.id }, context);
    expect(read.name).toBe("Core");
});
