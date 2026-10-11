import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { check, sql, unique } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { type CallInput, defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";
import { userContext } from "./fixture/user.ts";

/** The public space containing the handles. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** A private space nobody reads. */
const privateId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** The authentication creating a handle asks for: several factors within ten minutes. */
const RECENT = { assurance: 2, maxAge: 10 * 60 * 1000 };

/** Names unique within a space, created only by the user they refer to after strong authentication. */
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

        // serve the handles to the user each call refers to
        let level = 2;
        const store = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: { handle },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: handle.package,
                service: "test",
            },
        });
        const create = (
            user: string,
            input: { readonly holderId: string; readonly name: string; readonly id?: string },
            scope = spaceId,
        ) => {
            const context = userContext(user, scope, {
                assurance: { level, authenticatedAt: Date.now() },
            });

            return store
                .mutate(context.caller(), {
                    id: RequestId.create(),
                    calls: [handle.call("create", { spaceId: scope, ...input })],
                })
                .then(([written]) => written);
        };

        // take a name and an identifier as alice
        const taken = await create("alice", { holderId: "alice", name: "notes" });

        // tell bob of the taken name and identifier only for a handle he may create
        expect(await refusal(create("bob", { holderId: "bob", name: "notes" }))).toEqual([
            "CONFLICT",
            "another handle has the same scope and name",
        ]);
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
    const store = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
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
    const created = await store
        .mutate(context.caller(), {
            id: RequestId.create(),
            calls: [team.call("create", { spaceId, name: "Core" })],
        })
        .then(([written]) => written);
    const read = await store.query(context.caller(), team.call("get", { spaceId, id: created.id }));
    expect(read.name).toBe("Core");
});

test("run only the one method a Loan lends its caller, refusing every other", async () => {
    const storage = await TestDatabase.create("sqlite", [journal, ...team.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: { team },
        database: storage.database,
        callKey: testCallKey,
        origin: { package: team.package, service: "test" },
    });

    // create as a caller lent the create method alone, then read and create again
    const loaned = (method: string) =>
        userContext("alice", spaceId, {
            loan: {
                id: "loan-1",
                issuer: privateId,
                method: `${team.package.name}/team.${method}`,
            },
        });
    const created = await store
        .mutate(loaned("create").caller(), {
            id: RequestId.create(),
            calls: [team.call("create", { spaceId, name: "Core" })],
        })
        .then(([written]) => written);
    const read = await refusal(
        store.query(loaned("create").caller(), team.call("get", { spaceId, id: created.id })),
    );

    expect([created.name, read]).toEqual([
        "Core",
        [
            "FORBIDDEN",
            `the loan lends ${team.package.name}/team.create, not ${team.package.name}/team.get`,
        ],
    ]);
});

/** Ways to reach a team: an email address, or a phone number with its region. */
const contact = defineObject({
    name: "contact",
    plural: "contacts",
    scope: space,
    fields: {
        label: field.string(schema.string().min(1)),
        kind: field.enum(["email", "phone"]),
        address: field.string(schema.email()).optional(),
        number: field.string(schema.e164()).optional(),
        region: field.string(schema.string().length(2)).optional(),
    },
    relations: { owner: { subjects: [principal.user], grantedBy: "own" } },
    permissions: { read: relation("owner"), update: relation("owner"), own: relation("owner") },
    methods: (method) => ({
        get: method.get("read"),
        create: method.create("update", {
            fields: ["label"],
            variants: { kind: { email: ["address"], phone: ["number", "region"] } },
            creator: "owner",
        }),
    }),
    constraints: (entry) => [
        check("contact_kind", {
            OR: [
                {
                    kind: "email",
                    address: { isNotNull: true },
                    number: { isNull: true },
                    region: { isNull: true },
                },
                {
                    kind: "phone",
                    address: { isNull: true },
                    number: { isNotNull: true },
                    region: { isNotNull: true },
                },
            ],
        }),
    ],
});

test("create each variant of a sum type from its own fields beside the shared ones, refusing another variant's field", async () => {
    const storage = await TestDatabase.create("sqlite", [journal, ...contact.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: { contact },
        database: storage.database,
        callKey: testCallKey,
        origin: { package: contact.package, service: "test" },
    });
    const create = (input: CallInput<typeof contact, "create">) =>
        store
            .mutate(userContext("alice", spaceId).caller(), {
                id: RequestId.create(),
                calls: [contact.call("create", { spaceId, ...input })],
            })
            .then(([written]) => written);

    // create an email and a phone contact, each with its variant's columns alone
    const email = await create({ label: "Mail", kind: "email", address: "alice@example.com" });
    const phone = await create({
        label: "Desk",
        kind: "phone",
        number: "+4930123456",
        region: "de",
    });

    // refuse an email contact carrying a phone's number
    const mixed = await refusal(
        store
            .mutate(userContext("alice", spaceId).caller(), {
                id: RequestId.create(),
                calls: [
                    {
                        method: "contact.create",
                        release: contact.package.version,
                        input: {
                            spaceId,
                            label: "Both",
                            kind: "email",
                            address: "alice@example.com",
                            number: "+4930123456",
                        },
                    },
                ],
            })
            .then(([written]) => written),
    );

    // compare each contact's columns and the refusal
    const columns = [email, phone].map(({ label, kind, address, number, region }) => ({
        label,
        kind,
        address,
        number,
        region,
    }));
    expect([...columns, mixed]).toEqual([
        { label: "Mail", kind: "email", address: "alice@example.com", number: null, region: null },
        { label: "Desk", kind: "phone", address: null, number: "+4930123456", region: "de" },
        ["BAD_REQUEST", 'invalid input: unrecognized key: "number"'],
    ]);
});
