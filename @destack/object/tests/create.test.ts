import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Caller, none, principal, relation } from "@destack/access";

import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Clock, Identifier, schema } from "@destack/schema";

import { RequestId } from "@destack/runtime/request";
import { type CallInput, defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users } from "./fixture/user.ts";

/** The public space containing the handles. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** A private space nobody reads. */
const privateId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** The authentication creating a handle asks for: several factors within ten minutes. */
const RECENT = { assurance: 2, maxAge: 10 * 60 * 1000 };

/** Names unique within a space, created by their own user after strong authentication. */
const handle = defineObject({
    name: "handle",
    plural: "handles",
    scope: space,
    fields: {
        holder: field.reference(principal.user),
        name: field.string(schema.string().min(1)),
    },
    permissions: { read: relation("holder"), create: relation("holder") },
    elevated: { create: RECENT },
    constraints: {
        name: { kind: "unique", on: ["scope", "name"] },
    },
}).methods((method) => ({ create: method.create("create") }));

test.each(TEST_DIALECTS)(
    "report a create's conflicts only to callers the create permission admits, inside spaces they read, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [...handle.tables, ...callTables], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        await openSpace(storage.database, privateId, "private");

        // serve the handles to the user each call refers to
        let level = 2;
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([handle]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: {
                package: handle.package,
            },
        });
        const create = (
            user: Identifier<"user">,
            input: { readonly holderId: string; readonly name: string; readonly id?: string },
            scope = spaceId,
        ) => {
            const context = userContext(user, scope, {
                assurance: { level, authenticatedAt: Date.now() },
            });

            return store
                .mutate(context.caller(), {
                    id: RequestId.create(Date.now()),
                    calls: [handle.call("create", { spaceId: scope, ...input })],
                })
                .then(([written]) => written);
        };

        // take a name and an identifier as alice
        const taken = await create(users.alice, { holderId: users.alice, name: "notes" });

        // tell bob of the taken name and identifier only for a handle he may create
        expect(await refusal(create(users.bob, { holderId: users.bob, name: "notes" }))).toEqual([
            "CONFLICT",
            "another handle has the same scope and name",
        ]);
        await expect(
            create(users.bob, { holderId: users.bob, name: "tasks", id: taken.id }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "handle identifier is taken" });

        // find nothing for a handle bob may not create, taken or free
        const refusals: unknown[] = [];
        for (const input of [
            { holderId: users.alice, name: "notes" },
            { holderId: users.alice, name: "tasks", id: taken.id },
            { holderId: users.alice, name: "tasks" },
        ]) {
            refusals.push(await refusal(create(users.bob, input)));
        }
        expect(refusals).toEqual([
            ["NOT_FOUND", "handle not found"],
            ["NOT_FOUND", "handle not found"],
            ["NOT_FOUND", "handle not found"],
        ]);

        // challenge bob before a conflict, and find nothing for alice's handle
        level = 1;
        await expect(
            create(users.bob, { holderId: users.bob, name: "notes" }),
        ).rejects.toMatchObject({
            code: "INSUFFICIENT_AUTHENTICATION",
            stepUp: RECENT,
        });
        await expect(
            create(users.bob, { holderId: users.alice, name: "notes" }),
        ).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "handle not found",
        });
        level = 2;

        // create a handle in a space the caller cannot read, as its permission admits
        const admitted = await create(
            users.alice,
            { holderId: users.alice, name: "notes" },
            privateId,
        );
        expect({ holderId: admitted.holderId, name: admitted.name }).toEqual({
            holderId: users.alice,
            name: "notes",
        });
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
}).methods((method) => ({
    create: method.create("update", { fields: ["name"], creator: "owner" }),
}));

test("create an object the caller has the permission on only as the creator its creation relates", async () => {
    const storage = await TestDatabase.create("sqlite", [...callTables, ...team.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([team]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: {
            package: team.package,
        },
    });
    const context = userContext(users.alice, spaceId);

    // create a team as its owner and read it as that owner
    const created = await store
        .mutate(context.caller(), {
            id: RequestId.create(Date.now()),
            calls: [team.call("create", { spaceId, name: "Core" })],
        })
        .then(([written]) => written);
    const read = await store.query(context.caller(), team.call("get", { spaceId, id: created.id }));
    expect(read.name).toBe("Core");
});

test("run only the one method a one-method grant reaches, refusing every other method of the type", async () => {
    const storage = await TestDatabase.create("sqlite", [...callTables, ...team.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([team]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: team.package },
    });

    // create through a token granted only the create method, then read
    const grantId = Identifier.create("grant");
    const granted = userContext(users.alice, spaceId, {
        delegates: [principal.personalAccessToken.reference(users.alice, "token-alice")],
        grant: {
            id: grantId,
            scope: users.alice,
            permissions: [
                { ...team.permission("update"), scope: spaceId, method: "create" },
                { ...team.permission("read"), scope: spaceId, method: "create" },
            ],
        },
    });
    const created = await store
        .mutate(granted.caller(), {
            id: RequestId.create(Date.now()),
            calls: [team.call("create", { spaceId, name: "Core" })],
        })
        .then(([written]) => written);
    const read = await refusal(
        store.query(granted.caller(), team.call("get", { spaceId, id: created.id })),
    );

    expect([created.name, read]).toEqual([
        "Core",
        ["FORBIDDEN", `grant ${grantId} does not allow team.get in ${spaceId}`],
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
    constraints: {
        kind: {
            kind: "check",
            condition: {
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
            },
        },
    },
}).methods((method) => ({
    create: method.create("update", {
        fields: ["label"],
        variants: { kind: { email: ["address"], phone: ["number", "region"] } },
        creator: "owner",
    }),
}));

test("create each variant of a sum type from its own fields beside the shared ones, refusing another variant's field", async () => {
    const storage = await TestDatabase.create("sqlite", [...callTables, ...contact.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([contact]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: contact.package },
    });
    const create = (input: CallInput<typeof contact, "create">) =>
        store
            .mutate(userContext(users.alice, spaceId).caller(), {
                id: RequestId.create(Date.now()),
                calls: [contact.call("create", { spaceId, ...input })],
            })
            .then(([written]) => written);

    // create an email and a phone contact, each with only its variant's columns
    const email = await create({ label: "Mail", kind: "email", address: "alice@example.com" });
    const phone = await create({
        label: "Desk",
        kind: "phone",
        number: "+4930123456",
        region: "de",
    });

    // refuse an email contact with a phone's number
    const mixed = await refusal(
        store
            .mutate(userContext(users.alice, spaceId).caller(), {
                id: RequestId.create(Date.now()),
                calls: [
                    {
                        type: contact.typeReference,
                        method: "create",
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

/** Tickets filed in the queue their caller names, else in triage, as their creation fills it. */
const ticket = defineObject({
    name: "ticket",
    plural: "tickets",
    scope: space,
    fields: { title: field.string(), queue: field.string() },
    permissions: { read: none(), write: none() },
}).methods((method) => ({ create: method.create("write", { filled: ["queue"] }) }));

test("create an object without a field its creation fills, keeping the field the caller gives", async () => {
    // serve tickets whose creation fills the queue a caller leaves out
    const storage = await TestDatabase.create("sqlite", [...callTables, ...ticket.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([
            ticket.implement({
                methods: {
                    create: (call, next) =>
                        next(
                            call.with({
                                input: { ...call.input, queue: call.input.queue ?? "triage" },
                            }),
                        ),
                },
            }),
        ]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: ticket.package },
    });
    const create = (input: CallInput<typeof ticket, "create">) =>
        store
            .mutate(Caller.of(spacePrincipal(spaceId), Date.now()), {
                id: RequestId.create(Date.now()),
                calls: [ticket.call("create", { spaceId, ...input })],
            })
            .then(([written]) => written);

    // file one ticket without a queue and one in billing
    const filed = [
        await create({ title: "Login" }),
        await create({ title: "Invoice", queue: "billing" }),
    ];

    expect(filed.map(({ title, queue }) => [title, queue])).toEqual([
        ["Login", "triage"],
        ["Invoice", "billing"],
    ]);
});

/** Channels keyed by the slug their holder chooses. */
const channel = defineObject({
    name: "channel",
    plural: "channels",
    scope: space,
    key: schema.string().regex(/^[a-z]+$/u),
    fields: { holder: field.reference(principal.user), name: field.string() },
    permissions: { read: relation("holder"), create: relation("holder") },
}).methods((method) => ({ create: method.create("create") }));

test("create an object under a natural key the creator chooses, refusing a taken one", async () => {
    const storage = await TestDatabase.create("sqlite", [...callTables, ...channel.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([channel]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: channel.package },
    });
    const context = userContext(users.alice, spaceId);
    const create = (name: string) =>
        store
            .mutate(context.caller(), {
                id: RequestId.create(Date.now()),
                calls: [
                    channel.call("create", { spaceId, id: "core", holderId: users.alice, name }),
                ],
            })
            .then(([written]) => written.id);

    // keep the chosen key, and refuse it once taken
    expect([await create("Core"), await refusal(create("Again"))]).toEqual([
        "core",
        ["CONFLICT", "channel identifier is taken"],
    ]);
});
