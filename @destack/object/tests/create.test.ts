import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { AccessContext, Caller, none, principal, relation } from "@destack/access";
import { TestAccess } from "@destack/access/test";

import { Transaction } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Identifier, schema } from "@destack/schema";

import { RequestId, ObjectReference, Scope } from "@destack/sync";
import { type MethodInput, defineObject, field } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The public space containing the handles. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** A private space nobody reads. */
const privateId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** The authentication creating a handle asks for: several factors within ten minutes. */
const RECENT = { assurance: 2, maxAge: 10 * 60 * 1000 };

/** Names unique within a space, created by their user after strong authentication. */
const handle = defineObject(
    {
        name: "handle",
        plural: "handles",
        title: t`${plural(argument("count"), { one: "Handle", other: "Handles" })}`,
        scope: space,
        fields: {
            holder: field.reference(principal.user),
            name: field.string(schema.string().min(1)),
        },
        constraints: {
            name: { kind: "unique", on: ["scope", "name"] },
        },
        access: {
            permissions: { read: relation("holder"), create: relation("holder") },
            elevated: { create: RECENT },
        },
    },
    (method) => ({
        create: method.create("create"),
        claim: method.create("create", { onConflict: "name" }),
        adopt: method.create("create", { onConflict: "id" }),
    }),
);

test("report a create's conflicts only to callers the create permission allows, inside spaces they read", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { handle }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    await openSpace(storage.database, world, privateId, "private");

    // serve the handles to the user each call refers to
    let level = 2;
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([handle]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
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
            environment: world,
            assurance: { level, authenticatedAt: world.clock.now() },
        });

        return store
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [handle.call("create", { spaceId: scope, ...input })],
            })
            .then(([written]) => written);
    };

    // take a name and an identifier as alice
    const taken = await create(users.alice, { holderId: users.alice, name: "notes" });

    // tell bob of the taken name and identifier only for a handle he may create
    expect(await refusal(create(users.bob, { holderId: users.bob, name: "notes" }))).toEqual([
        "CONFLICT",
        "OBJECT_EXISTS",
        "'handle' with the same 'scope and name' already exists",
    ]);
    expect(
        await refusal(create(users.bob, { holderId: users.bob, name: "tasks", id: taken.id })),
    ).toEqual(["CONFLICT", "ID_EXISTS", "'handle' identifier already exists"]);

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
        ["NOT_FOUND", "NOT_FOUND", "handle not found"],
        ["NOT_FOUND", "NOT_FOUND", "handle not found"],
        ["NOT_FOUND", "NOT_FOUND", "handle not found"],
    ]);

    // challenge bob before a conflict, and find nothing for alice's handle
    level = 1;
    await expect(create(users.bob, { holderId: users.bob, name: "notes" })).rejects.toMatchObject({
        reason: "ASSURANCE_UNAUTHENTICATED",
        data: RECENT,
    });
    await expect(create(users.bob, { holderId: users.alice, name: "notes" })).rejects.toMatchObject(
        {
            code: "NOT_FOUND",
            message: "handle not found",
        },
    );
    level = 2;

    // create a handle in a space the caller cannot read, as its permission allows
    const allowed = await create(users.alice, { holderId: users.alice, name: "notes" }, privateId);
    expect({ holderId: allowed.holderId, name: allowed.name }).toEqual({
        holderId: users.alice,
        name: "notes",
    });
});

/** Teams any caller creates as owner, updated only by owners. */
const team = defineObject(
    {
        name: "team",
        plural: "teams",
        title: t`${plural(argument("count"), { one: "Team", other: "Teams" })}`,
        scope: space,
        fields: { name: field.string(schema.string().min(1)) },
        access: {
            relations: { owner: { subjects: [principal.user], grantedBy: "manage" } },
            permissions: {
                read: relation("owner"),
                update: relation("owner"),
                manage: relation("owner"),
            },
        },
    },
    (method) => ({
        create: method.create("update", { fields: ["name"], creator: "owner" }),
    }),
);

test("answer the object a creation's unique constraint matches on a conflict, to its creator and no one else", async () => {
    // serve the handles in a public space
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { handle }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([handle]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: handle.package },
    });
    const call = (
        user: Identifier<"user">,
        method: "claim" | "adopt",
        input: { readonly name: string; readonly id?: string },
    ) => {
        const context = userContext(user, spaceId, {
            environment: world,
            assurance: { level: 2, authenticatedAt: world.clock.now() },
        });

        return store
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [handle.call(method, { spaceId, holderId: user, ...input })],
            })
            .then(([written]) => written);
    };

    // claim a name twice as alice, then as bob, and adopt one identifier twice
    const first = await call(users.alice, "claim", { name: "notes" });
    const again = await call(users.alice, "claim", { name: "notes" });
    const bob = await refusal(call(users.bob, "claim", { name: "notes" }));
    const adopted = await call(users.alice, "adopt", { id: first.id, name: "tasks" });
    const rows = await storage.database.select().from(handle.table);

    // answer alice's handle again, by its name and its identifier, without another row; find nothing for bob
    expect({
        same: [again.id, adopted.id].map((id) => id === first.id),
        names: rows.map((row) => row.name),
        bob,
    }).toEqual({
        same: [true, true],
        names: ["notes"],
        bob: ["NOT_FOUND", "NOT_FOUND", "handle not found"],
    });
});

test("create an object the caller has the permission on only as the creator its creation relates", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { team }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([team]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: {
            package: team.package,
        },
    });
    const context = userContext(users.alice, spaceId, { environment: world });

    // create a team as its owner and read it as that owner
    const created = await store
        .mutate(context.access(), {
            id: RequestId.create(world),
            calls: [team.call("create", { spaceId, name: "Core" })],
        })
        .then(([written]) => written);
    const read = await store.query(context.access(), team.call("get", { spaceId, id: created.id }));
    expect(read.name).toBe("Core");
});

test("run only the one method a one-method grant includes, refusing every other method of the type", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { team }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([team]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: team.package },
    });

    // create through a token granted only the create method, then read
    const token = principal.client.reference(Scope.universe.id, "client-alice");
    const claim = await new TestAccess(storage.database, world).replicateGrant(
        principal.user.reference("universe", users.alice),
        token,
        [
            { ...team.permission("update"), scope: spaceId, method: "create" },
            { ...team.permission("read"), scope: spaceId, method: "create" },
        ],
    );
    const granted = userContext(users.alice, spaceId, {
        environment: world,
        actors: [token],
        grants: [claim],
    });
    const created = await store
        .mutate(granted.access(), {
            id: RequestId.create(world),
            calls: [team.call("create", { spaceId, name: "Core" })],
        })
        .then(([written]) => written);
    const read = await refusal(
        store.query(granted.access(), team.call("get", { spaceId, id: created.id })),
    );

    expect([created.name, read]).toEqual([
        "Core",
        [
            "FORBIDDEN",
            "GRANT_REFUSES_METHOD_DENIED",
            `'team.get' in '${spaceId}' is forbidden: grant '${claim.id}' does not allow it`,
        ],
    ]);
});

/** Ways to contact a team: an email address, or a phone number with its region. */
const contact = defineObject(
    {
        name: "contact",
        plural: "contacts",
        title: t`${plural(argument("count"), { one: "Contact", other: "Contacts" })}`,
        scope: space,
        fields: {
            label: field.string(schema.string().min(1)),
            kind: field.enum(["email", "phone"]),
            address: field.string(schema.email()).optional(),
            number: field.string(schema.e164()).optional(),
            region: field.string(schema.string().length(2)).optional(),
        },
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
        access: {
            relations: { owner: { subjects: [principal.user], grantedBy: "manage" } },
            permissions: {
                read: relation("owner"),
                update: relation("owner"),
                manage: relation("owner"),
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

test("create each variant of a sum type from its fields beside the shared ones, refusing another variant's field", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { contact }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([contact]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: contact.package },
    });
    const create = (input: MethodInput<typeof contact, "create">) =>
        store
            .mutate(userContext(users.alice, spaceId, { environment: world }).access(), {
                id: RequestId.create(world),
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
            .mutate(userContext(users.alice, spaceId, { environment: world }).access(), {
                id: RequestId.create(world),
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
        ["BAD_REQUEST", "INPUT_INVALID", 'invalid input: unrecognized key: "number"'],
    ]);
});

/** Tickets filed in the queue their caller names, else in triage, as their creation fills it. */
const ticket = defineObject(
    {
        name: "ticket",
        plural: "tickets",
        title: t`${plural(argument("count"), { one: "Ticket", other: "Tickets" })}`,
        scope: space,
        fields: { title: field.string(), queue: field.string() },
        access: {
            permissions: { read: none(), write: none() },
        },
    },
    (method) => ({ create: method.create("write", { filled: ["queue"] }) }),
);

test("create an object without a field its creation fills, taking the field the caller gives", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve tickets whose creation fills the queue a caller leaves out
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { ticket }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
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
        events: await testEvents(storage.database, world),
        origin: { package: ticket.package },
    });
    const create = (input: MethodInput<typeof ticket, "create">) =>
        store
            .mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()), {
                id: RequestId.create(world),
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
        title: t`${plural(argument("count"), { one: "Channel", other: "Channels" })}`,
        scope: space,
        key: "slug",
        fields: {
            slug: field.string(schema.string().regex(/^[a-z]+$/u)).immutable(),
            holder: field.reference(principal.user),
            name: field.string(),
        },
        access: {
            permissions: { read: relation("holder"), create: relation("holder") },
        },
    },
    (method) => ({ create: method.create("create"), get: method.get("read") }),
);

/** Posts in a channel, which their author names by the channel's slug. */
const post = defineObject(
    {
        name: "post",
        plural: "posts",
        title: t`${plural(argument("count"), { one: "Post", other: "Posts" })}`,
        scope: space,
        fields: {
            channel: field.reference(channel),
            holder: field.reference(principal.user),
            text: field.string(),
        },
        access: {
            permissions: { read: relation("holder"), create: relation("holder") },
        },
    },
    (method) => ({ create: method.create("create") }),
);

test("create an object under a natural key the creator chooses, read it by that key, and refuse a taken one", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { channel, post }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([channel, post]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: channel.package },
    });
    const context = userContext(users.alice, spaceId, { environment: world });
    const create = (name: string) =>
        store
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [
                    channel.call("create", { spaceId, slug: "core", holderId: users.alice, name }),
                ],
            })
            .then(([written]) => written);

    // take the chosen key, read the channel by it, and refuse the key once taken
    const created = await create("Core");
    const read = await store.query(context.access(), channel.call("get", { spaceId, id: "core" }));
    // post into the channel by its slug, storing its identifier, and refuse a slug nobody holds
    const write = (slug: string) =>
        store.mutate(context.access(), {
            id: RequestId.create(world),
            calls: [
                post.call("create", {
                    spaceId,
                    channelId: slug,
                    holderId: users.alice,
                    text: "Hi",
                }),
            ],
        });
    const [posted] = await write("core");
    expect([
        [read.id, read.slug, read.name],
        channel.identifies(created.id),
        await refusal(create("Again")),
        posted.channelId,
        await refusal(write("missing")),
    ]).toEqual([
        [created.id, "core", "Core"],
        true,
        ["CONFLICT", "OBJECT_EXISTS", "'channel' with the same 'scope and slug' already exists"],
        created.id,
        ["NOT_FOUND", "REFERENCE_NOT_FOUND", "'channel' with key 'missing' not found"],
    ]);
});

/** Desks keyed by a name only open desks hold, which closing one frees. */
const desk = defineObject(
    {
        name: "desk",
        plural: "desks",
        title: t`${plural(argument("count"), { one: "Desk", other: "Desks" })}`,
        scope: space,
        key: "name",
        fields: {
            name: field.string(),
            holder: field.reference(principal.user),
            closedAt: field.time().optional(),
        },
        constraints: {
            openName: {
                kind: "index",
                on: ["scope", "name"],
                unique: true,
                where: { closedAt: { isNull: true } },
            },
        },
        access: {
            permissions: { read: relation("holder"), write: relation("holder") },
        },
    },
    (method) => ({
        create: method.create("write"),
        get: method.get("read"),
        update: method.update("write", { fields: ["closedAt"] }),
    }),
);

test("address an object by a key its unique index holds only for open objects, the closed one keeping its name", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { desk }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([desk]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: desk.package },
    });
    const access = userContext(users.alice, spaceId, { environment: world }).access();
    const open = () =>
        store
            .mutate(access, {
                id: RequestId.create(world),
                calls: [desk.call("create", { spaceId, name: "front", holderId: users.alice })],
            })
            .then(([written]) => written);

    // open a desk, close it by its name, then open another under the freed name
    const first = await open();
    await store.mutate(access, {
        id: RequestId.create(world),
        calls: [desk.call("update", { spaceId, id: "front", closedAt: 1 })],
    });
    const second = await open();
    const read = await store.query(access, desk.call("get", { spaceId, id: "front" }));

    // expect the name to reach the open desk alone
    expect([read.id === second.id, read.id === first.id]).toEqual([true, false]);
});

/** Topics of a space, which a holder creates. */
const topic = defineObject(
    {
        name: "topic",
        plural: "topics",
        title: t`${plural(argument("count"), { one: "Topic", other: "Topics" })}`,
        scope: space,
        fields: { holder: field.reference(principal.user), name: field.string() },
        access: {
            permissions: { read: relation("holder"), create: relation("holder") },
        },
    },
    (method) => ({ create: method.create("create") }),
);

test("create an object once for a request two transactions run, the one reading before the other committed running again", async () => {
    // run in a world of its own, moving its time while the work waits
    const simulation = testSimulation();
    const world = simulation.environment("test");
    await (async () => {
        const storage = await TestDatabase.create(
            tablesOf({
                packages: [objectPackage, { objects: { topic }, tables: [] }],
                follows: [],
                tables: [],
            }).tables,
            { environment: world, isMigrated: true },
        );
        onTestFinished(() => storage.close());
        await openSpace(storage.database, world, spaceId);
        const store = TestObjectStore.of({
            chain: async () => undefined,
            environment: world,
            principal: spacePrincipal(spaceId),
            servers: serversOf([topic]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            origin: { package: topic.package },
        });

        // create the topic unless the transaction reads it, both transactions under one request
        const caller = userContext(users.alice, spaceId, { environment: world }).access();
        const id = RequestId.create(world);
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
    })();
});

/** Watches of a space's objects, by their watcher, which code adds for others too. */
const watch = defineObject(
    {
        name: "watch",
        plural: "watches",
        title: t`${plural(argument("count"), { one: "Watch", other: "Watches" })}`,
        scope: space,
        fields: { watcher: field.subject().caller(), topic: field.string() },
        access: {
            permissions: { read: relation("watcher"), add: none() },
        },
    },
    (method) => ({
        create: method.create("read"),
        add: method.create("add", { visibility: "package" }),
    }),
);

test("fill a subject field from the caller, and write all its columns in a server-only creation", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the watches to the space's work
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { watch }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([watch]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: watch.package },
    });
    const caller = AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now());
    const ada = principal.user.reference(Scope.universe.id, users.alice);

    // watch a topic as the space, and add a watch of another topic for ada
    const [watched, added] = await store.mutate(caller, {
        id: RequestId.create(world),
        calls: [
            watch.call("create", { spaceId, topic: "Launch" }),
            watch.call("add", {
                spaceId,
                topic: "Budget",
                ...ObjectReference.columns("watcher", ada),
            }),
        ],
    });

    // the creation records the caller, and the server-only one records ada with every column
    expect([ObjectReference.at("watcher", watched), ObjectReference.at("watcher", added)]).toEqual([
        spacePrincipal(spaceId),
        ada,
    ]);
});
