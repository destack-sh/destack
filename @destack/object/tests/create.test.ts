import { expect, onTestFinished, refusal, test } from "@destack/test";
import { AccessContext, Caller, none, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";

import { Transaction } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Identifier, schema } from "@destack/schema";

import { RequestId, ObjectReference, Scope } from "@destack/sync";
import { type MethodInput, defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users } from "./fixture/user.ts";
import { Clock } from "@destack/environment";

/** The public space containing the handles. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** A private space nobody reads. */
const privateId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** The authentication creating a handle asks for: several factors within ten minutes. */
const RECENT = { assurance: 2, maxAge: 10 * 60 * 1000 };

/** Names unique within a space, created by their own user after strong authentication. */
const handle = defineObject(
    {
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
    },
    (method) => ({ create: method.create("create") }),
);

test.each(TEST_DIALECTS)(
    "report a create's conflicts only to callers the create permission allows, inside spaces they read, on %s",
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
                .mutate(context.access(), {
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

        // create a handle in a space the caller cannot read, as its permission allows
        const allowed = await create(
            users.alice,
            { holderId: users.alice, name: "notes" },
            privateId,
        );
        expect({ holderId: allowed.holderId, name: allowed.name }).toEqual({
            holderId: users.alice,
            name: "notes",
        });
    },
);

/** Teams any caller creates and owns, updated only by owners. */
const team = defineObject(
    {
        name: "team",
        plural: "teams",
        scope: space,
        fields: { name: field.string(schema.string().min(1)) },
        relations: { owner: { subjects: [principal.user], grantedBy: "own" } },
        permissions: { read: relation("owner"), update: relation("owner"), own: relation("owner") },
    },
    (method) => ({
        create: method.create("update", { fields: ["name"], creator: "owner" }),
    }),
);

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
        .mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [team.call("create", { spaceId, name: "Core" })],
        })
        .then(([written]) => written);
    const read = await store.query(context.access(), team.call("get", { spaceId, id: created.id }));
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
    const token = principal.client.reference(Scope.universe.id, "client-alice");
    const grantId = await new AccessFixture(storage.database).replicateGrant(
        principal.user.reference("universe", users.alice),
        token,
        [
            { ...team.permission("update"), scope: spaceId, method: "create" },
            { ...team.permission("read"), scope: spaceId, method: "create" },
        ],
    );
    const granted = userContext(users.alice, spaceId, {
        actors: [token],
        grants: [{ id: grantId, scope: users.alice }],
    });
    const created = await store
        .mutate(granted.access(), {
            id: RequestId.create(Date.now()),
            calls: [team.call("create", { spaceId, name: "Core" })],
        })
        .then(([written]) => written);
    const read = await refusal(
        store.query(granted.access(), team.call("get", { spaceId, id: created.id })),
    );

    expect([created.name, read]).toEqual([
        "Core",
        ["FORBIDDEN", `grant ${grantId} does not allow team.get in ${spaceId}`],
    ]);
});

/** Ways to reach a team: an email address, or a phone number with its region. */
const contact = defineObject(
    {
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
    },
    (method) => ({
        create: method.create("update", {
            fields: ["label"],
            variants: { kind: { email: ["address"], phone: ["number", "region"] } },
            creator: "owner",
        }),
    }),
);

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
    const create = (input: MethodInput<typeof contact, "create">) =>
        store
            .mutate(userContext(users.alice, spaceId).access(), {
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
            .mutate(userContext(users.alice, spaceId).access(), {
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
const ticket = defineObject(
    {
        name: "ticket",
        plural: "tickets",
        scope: space,
        fields: { title: field.string(), queue: field.string() },
        permissions: { read: none(), write: none() },
    },
    (method) => ({ create: method.create("write", { filled: ["queue"] }) }),
);

test("create an object without a field its creation fills, taking the field the caller gives", async () => {
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
    const create = (input: MethodInput<typeof ticket, "create">) =>
        store
            .mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()), {
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
const channel = defineObject(
    {
        name: "channel",
        plural: "channels",
        scope: space,
        key: schema.string().regex(/^[a-z]+$/u),
        fields: { holder: field.reference(principal.user), name: field.string() },
        permissions: { read: relation("holder"), create: relation("holder") },
    },
    (method) => ({ create: method.create("create") }),
);

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
            .mutate(context.access(), {
                id: RequestId.create(Date.now()),
                calls: [
                    channel.call("create", { spaceId, id: "core", holderId: users.alice, name }),
                ],
            })
            .then(([written]) => written.id);

    // take the chosen key, and refuse it once taken
    expect([await create("Core"), await refusal(create("Again"))]).toEqual([
        "core",
        ["CONFLICT", "channel identifier is taken"],
    ]);
});

/** Topics of a space, which a holder creates. */
const topic = defineObject(
    {
        name: "topic",
        plural: "topics",
        scope: space,
        fields: { holder: field.reference(principal.user), name: field.string() },
        permissions: { read: relation("holder"), create: relation("holder") },
    },
    (method) => ({ create: method.create("create") }),
);

test.each(TEST_DIALECTS)(
    "create an object once for a request two transactions run, the one reading before the other committed running again, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [...callTables, ...topic.tables], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([topic]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: { package: topic.package },
        });

        // create the topic unless the transaction reads it, both transactions under one request
        const caller = userContext(users.alice, spaceId).access();
        const id = RequestId.create(Date.now());
        const create = topic.call("create", { spaceId, holderId: users.alice, name: "ideas" });
        const step = (attempted: () => Promise<void>) =>
            Transaction.run(storage.database, async (transaction) => {
                const [found] = await transaction
                    .view()
                    .read(topic.table)
                    .findMany({ where: { name: "ideas" } });
                await attempted();
                if (found === undefined) {
                    await store.mutate(caller, { id, calls: [create] }, { transaction });
                }
            });

        // read in the later transaction, commit the first, then create in the later one
        const committed = Promise.withResolvers<void>();
        let attempts = 0;
        const later = step(async () => {
            attempts += 1;
            if (attempts === 1) {
                await committed.promise;
            }
        });
        await step(async () => {});
        committed.resolve();
        await later;

        // expect the one topic, the later transaction running again as it lost the race
        const rows = await storage.database.select({ name: topic.table.name }).from(topic.table);
        expect({ rows, attempts }).toEqual({ rows: [{ name: "ideas" }], attempts: 2 });
    },
);

/** Watches of a space's objects, by their watcher, which code adds for others too. */
const watch = defineObject(
    {
        name: "watch",
        plural: "watches",
        scope: space,
        fields: { watcher: field.subject().caller(), topic: field.string() },
        permissions: { read: relation("watcher"), add: none() },
    },
    (method) => ({
        create: method.create("read"),
        add: method.create("add", { isInternal: true }),
    }),
);

test("fill a subject field from the caller, and write all its columns in an internal creation", async () => {
    // serve the watches to the space's own work
    const storage = await TestDatabase.create("sqlite", [...callTables, ...watch.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([watch]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: watch.package },
    });
    const own = AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now());
    const ada = principal.user.reference(Scope.universe.id, users.alice);

    // watch a topic as the space, and add a watch of another topic for ada
    const [watched, added] = await store.mutate(own, {
        id: RequestId.create(Date.now()),
        calls: [
            watch.call("create", { spaceId, topic: "Launch" }),
            watch.call("add", {
                spaceId,
                topic: "Budget",
                ...ObjectReference.columns("watcher", ada),
            }),
        ],
    });

    // the creation names the caller, and the internal one names ada with every column
    expect([ObjectReference.at("watcher", watched), ObjectReference.at("watcher", added)]).toEqual([
        spacePrincipal(spaceId),
        ada,
    ]);
});
