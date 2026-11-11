import { TestObjectStore } from "../src/test/index.ts";
import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testCallKey } from "@destack/object/test";
import { TestDatabase } from "@destack/db/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { AccessContext, Caller, none, principal, relation } from "@destack/access";

import { Scope, RequestId } from "@destack/sync";
import { asc, ControlLoop, eq, defineDatabase, now } from "@destack/db";
import { Failure } from "@destack/error";
import { schema, present, type Identifier } from "@destack/schema";

import { defineObject, field, ObjectWatch } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { auditedCalls, settleControl } from "../src/test/index.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The space with the reminders. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Reminders the space sends once due, a batch per topic. */
const reminder = defineObject(
    {
        name: "reminder",
        plural: "reminders",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            topic: field.string(),
            dueAt: field.time(),
            sentAt: field.time().optional(),
        },
        permissions: { read: relation("owner"), write: relation("owner"), send: none() },
    },
    (method) => ({
        create: method.create("write", { fields: ["topic", "dueAt"] }),
        send: method.mutation({ permission: "send", visibility: "package" }),
    }),
).implement({
    methods: { send: (call) => call.update({ sentAt: call.now }) },
    controllers: [
        {
            pending: { sentAt: { isNull: true }, dueAt: { lte: now } },
            key: ["topic"],
            // send the due reminders of a topic together
            reconcile: async ({ rows, mutate }) => {
                await mutate(rows.map((row) => reminder.call("send", reminder.target(row))));
            },
        },
    ],
});

/** Gates whose controller fails as their outcome field says, until it opens them. */
const gate = defineObject(
    {
        name: "gate",
        plural: "gates",
        scope: space,
        controlled: true,
        fields: {
            owner: field.reference(principal.user).caller(),
            outcome: field.string(),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", { fields: ["outcome"] }),
        update: method.update("write", { fields: ["outcome"] }),
    }),
).implement({
    controllers: [
        {
            pending: { generation: { gt: { field: "syncedGeneration" } } },
            // refuse or fail each gate as its outcome says
            reconcile: async ({ rows }) => {
                for (const row of rows) {
                    if (row.outcome === "refused") {
                        throw Failure.error({
                            code: "FORBIDDEN",
                            reason: "PERMISSION_DENIED",
                            domain: "@destack/access",
                            message: "permission 'bind' is forbidden: the caller lacks it",
                        });
                    } else if (row.outcome === "unavailable") {
                        throw Failure.error(
                            Failure.http("SERVICE_UNAVAILABLE", "service unavailable"),
                        );
                    }
                }
            },
        },
    ],
});

/** Latches whose projection fails as their outcome field says. */
const latch = defineObject(
    {
        name: "latch",
        plural: "latches",
        scope: space,
        controlled: true,
        fields: {
            owner: field.reference(principal.user).caller(),
            outcome: field.string(),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", { fields: ["outcome"] }),
        update: method.update("write", { fields: ["outcome"] }),
    }),
).implement({
    projections: [
        {
            watches: [],
            // refuse a latch whose outcome says so
            project: async (_id, { rows }) => {
                if (rows.some((row) => row.outcome === "refused")) {
                    throw Failure.error({
                        code: "FORBIDDEN",
                        reason: "PERMISSION_DENIED",
                        domain: "@destack/access",
                        message: "permission 'bind' is forbidden: the caller lacks it",
                    });
                }
            },
        },
    ],
});

/** Switches a lever's projection reports on, which no controller of their own reconciles. */
const toggle = defineObject(
    {
        name: "toggle",
        plural: "toggles",
        scope: space,
        controlled: true,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({ create: method.create("write", { fields: [] }) }),
);

/** Levers whose projection fails as their outcome says, reporting Synced on their toggle. */
const lever = defineObject(
    {
        name: "lever",
        plural: "levers",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            toggle: field.reference(() => toggle),
            outcome: field.string(),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", { fields: ["toggleId", "outcome"] }),
        update: method.update("write", { fields: ["outcome"] }),
    }),
).implement({
    projections: [
        {
            watches: [],
            reports: { object: toggle, identifiers: (rows) => rows.map((row) => row.toggleId) },
            // refuse a lever whose outcome says so
            project: async (_id, { rows }) => {
                if (rows.some((row) => row.outcome === "refused")) {
                    throw Failure.error({
                        code: "FORBIDDEN",
                        reason: "PERMISSION_DENIED",
                        domain: "@destack/access",
                        message: "permission 'bind' is forbidden: the caller lacks it",
                    });
                }
            },
        },
    ],
});

/** The database with the reminders, their access and the journal. */
const reminderDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [
            objectPackage,
            { objects: { reminder, gate, latch, toggle, lever }, tables: [] },
        ],
        follows: [],
        tables: [],
    }),
});

test("reconcile a type's due objects together by the key its controller declares, waking when the next one is due", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const storage = await TestDatabase.create(reminderDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([reminder]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: reminder.package },
    });

    // insert two due reminders about tea, one later about tea, and one due about lunch
    const context = userContext(users.frank, spaceId, { environment: world });
    const start = world.clock.now();
    const create = async (topic: string, dueAt: number) =>
        store
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [reminder.call("create", { spaceId, topic, dueAt })],
            })
            .then(([written]) => written);
    await create("tea", start - 1000);
    await create("tea", start - 500);
    await create("tea", start + 60_000);
    await create("lunch", start - 1000);

    // send the due ones by topic, then the later one at its time
    const sent = async () =>
        (
            await database
                .select({ topic: reminder.table.topic, sentAt: reminder.table.sentAt })
                .from(reminder.table)
                .orderBy(asc(reminder.table.dueAt), asc(reminder.table.topic))
        ).map((row) => [row.topic, row.sentAt === null ? null : row.sentAt - start]);
    const sends = async () =>
        (await auditedCalls(database)).filter((call) => call.method === "reminder.send").length;
    await settleControl(store);
    const first = [await sent(), await sends()];
    await simulation.jump(start + 60_000);
    await settleControl(store);
    expect([first, await sent()]).toEqual([
        [
            [
                ["lunch", 0],
                ["tea", 0],
                ["tea", 0],
                ["tea", null],
            ],
            3,
        ],
        [
            ["lunch", 0],
            ["tea", 0],
            ["tea", 0],
            ["tea", 60_000],
        ],
    ]);
});

test("reconcile a pending object again as a watched table's change selects it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // keep every reminder pending, and select the first one again once any reminder changes
    const reconciled: string[] = [];
    let firstId = "";
    const watched = reminder.implement({
        controllers: [
            {
                pending: { sentAt: { isNull: true } },
                watches: [ObjectWatch.of(reminder.table, () => [{ id: firstId }])],
                reconcile: async ({ rows }) => {
                    reconciled.push(...rows.map((row) => row.topic));
                },
            },
        ],
    });
    const storage = await TestDatabase.create(reminderDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([watched]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: reminder.package },
    });
    const loop = new ControlLoop(database, store.control(), { report: () => {} });
    const stopping = new AbortController();
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // reconcile the first at its creation, then again beside the second as the second is created
    const create = async (topic: string) =>
        store
            .mutate(userContext(users.frank, spaceId, { environment: world }).access(), {
                id: RequestId.create(world),
                calls: [reminder.call("create", { spaceId, topic, dueAt: 1 })],
            })
            .then(([written]) => written);
    firstId = (await create("first")).id;
    await loop.idle();
    const created = new Set(reconciled.splice(0));
    await create("second");
    await loop.idle();

    // reconcile each object at least once per phase, a race of the start and the commit running one twice
    expect([[...created], [...new Set(reconciled)].toSorted()]).toEqual([
        ["first"],
        ["first", "second"],
    ]);
});

test("run a controller's calls as the space, audited with the controller's name, and refuse principals lacking the method's permission", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(reminderDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([reminder]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: reminder.package },
    });

    // send a due reminder of frank through the controller
    const due = await store
        .mutate(userContext(users.frank, spaceId, { environment: world }).access(), {
            id: RequestId.create(world),
            calls: [
                reminder.call("create", { spaceId, topic: "tea", dueAt: world.clock.now() - 1000 }),
            ],
        })
        .then(([written]) => written);
    await settleControl(store);
    const sends = (await auditedCalls(database)).filter((call) => call.method === "reminder.send");
    // refuse the send to its owner, who reads it, and conceal it from another user
    const [row] = await database.select().from(reminder.table);
    const send = (as: Identifier<"user">) =>
        refusal(
            store.mutate(
                AccessContext.of(
                    Caller.of(principal.user.reference(Scope.universe.id, as)),
                    world.clock.now(),
                ),
                {
                    id: RequestId.create(world),
                    calls: [reminder.call("send", reminder.target(present(row, "the reminder")))],
                },
            ),
        );

    // record the send as the space and the controller, and refuse both users
    expect([
        sends.map((call) => [call.execution.context.caller, call.execution.context.component]),
        await send(users.frank),
        await send(users.grace),
    ]).toEqual([
        [[{ subject: spacePrincipal(spaceId) }, "reminder"]],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission 'send' is forbidden: the caller lacks it"],
        ["NOT_FOUND", "NOT_FOUND", `reminder '${due.id}' not found`],
    ]);
});

test("record a lasting reconcile failure on a controlled object as its synced condition, sync it once reconciled, and keep a failing one pending", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(reminderDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([gate]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: gate.package },
    });
    const reported: unknown[] = [];
    const loop = new ControlLoop(database, store.control(), {
        report: (_controller, _key, error) => reported.push(error),
    });
    const stopping = new AbortController();
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // create a refused gate and an unavailable one, each failing its reconcile
    const access = userContext(users.frank, spaceId, { environment: world }).access();
    const create = async (outcome: string) =>
        store
            .mutate(access, {
                id: RequestId.create(world),
                calls: [gate.call("create", { spaceId, outcome })],
            })
            .then(([written]) => written);
    const refused = await create("refused");
    await create("unavailable");
    await loop.idle();
    const state = async () =>
        Object.fromEntries(
            (await database.select().from(gate.table).orderBy(asc(gate.table.outcome))).map(
                (row) => [row.outcome, [row.syncedGeneration, row.conditions]],
            ),
        );
    const recorded = await state();

    // open the refused gate, which its next reconcile syncs
    await store.mutate(access, {
        id: RequestId.create(world),
        calls: [gate.call("update", { ...gate.target(refused), outcome: "open" })],
    });
    await loop.idle();

    // record the refusal with its domain and reason, leave the outage unrecorded, and sync the opened gate
    const at = world.clock.now();
    expect({ recorded, synced: await state() }).toEqual({
        recorded: {
            refused: [
                0,
                {
                    Synced: {
                        status: "false",
                        observedGeneration: 1,
                        reason: "ReconcileError",
                        message:
                            "@destack/access PERMISSION_DENIED: permission 'bind' is forbidden: the caller lacks it",
                        lastTransitionAt: at,
                    },
                },
            ],
            unavailable: [0, {}],
        },
        synced: {
            open: [
                2,
                {
                    Synced: {
                        status: "true",
                        observedGeneration: 2,
                        reason: "ReconcileSuccess",
                        message: "",
                        lastTransitionAt: at,
                    },
                },
            ],
            unavailable: [0, {}],
        },
    });
});

test("record a lasting projection failure on a controlled object as its synced condition, and sync it once projected", async () => {
    // run in a world of its own, its latches projected by a running loop
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(reminderDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([latch]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: latch.package },
    });
    const loop = new ControlLoop(database, store.control(), { report: () => {} });
    const stopping = new AbortController();
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // create a refused latch, then open it
    const access = userContext(users.frank, spaceId, { environment: world }).access();
    const [created] = await store.mutate(access, {
        id: RequestId.create(world),
        calls: [latch.call("create", { spaceId, outcome: "refused" })],
    });
    await loop.idle();
    const state = async () => {
        const [row] = await database.select().from(latch.table);

        return [row?.syncedGeneration, row?.conditions];
    };
    const refused = await state();
    await store.mutate(access, {
        id: RequestId.create(world),
        calls: [
            latch.call("update", {
                ...latch.target(present(created, "the latch")),
                outcome: "open",
            }),
        ],
    });
    await loop.idle();

    // record the refusal on the latch, then sync it at its next generation
    const at = world.clock.now();
    expect({ refused, opened: await state() }).toEqual({
        refused: [
            0,
            {
                Synced: {
                    status: "false",
                    observedGeneration: 1,
                    reason: "ReconcileError",
                    message:
                        "@destack/access PERMISSION_DENIED: permission 'bind' is forbidden: the caller lacks it",
                    lastTransitionAt: at,
                },
            },
        ],
        opened: [
            2,
            {
                Synced: {
                    status: "true",
                    observedGeneration: 2,
                    reason: "ReconcileSuccess",
                    message: "",
                    lastTransitionAt: at,
                },
            },
        ],
    });
});

test("record a projection's lasting failure and its sync on the controlled object it reports on", async () => {
    // run in a world of its own, its levers projected by a running loop
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(reminderDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([toggle, lever]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: lever.package },
    });
    const loop = new ControlLoop(database, store.control(), { report: () => {} });
    const stopping = new AbortController();
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // create a toggle and a refused lever on it, then open the lever
    const access = userContext(users.frank, spaceId, { environment: world }).access();
    const [created] = await store.mutate(access, {
        id: RequestId.create(world),
        calls: [toggle.call("create", { spaceId })],
    });
    const toggleId = present(created, "the toggle").id;
    const [pulled] = await store.mutate(access, {
        id: RequestId.create(world),
        calls: [lever.call("create", { spaceId, toggleId, outcome: "refused" })],
    });
    await loop.idle();
    const state = async () => {
        const [row] = await database.select().from(toggle.table);

        return [row?.syncedGeneration, row?.conditions];
    };
    const refused = await state();
    await store.mutate(access, {
        id: RequestId.create(world),
        calls: [
            lever.call("update", {
                ...lever.target(present(pulled, "the lever")),
                outcome: "open",
            }),
        ],
    });
    await loop.idle();

    // record the refusal on the toggle at its generation, then sync it there
    const at = world.clock.now();
    expect({ refused, opened: await state() }).toEqual({
        refused: [
            0,
            {
                Synced: {
                    status: "false",
                    observedGeneration: 1,
                    reason: "ReconcileError",
                    message:
                        "@destack/access PERMISSION_DENIED: permission 'bind' is forbidden: the caller lacks it",
                    lastTransitionAt: at,
                },
            },
        ],
        opened: [
            1,
            {
                Synced: {
                    status: "true",
                    observedGeneration: 1,
                    reason: "ReconcileSuccess",
                    message: "",
                    lastTransitionAt: at,
                },
            },
        ],
    });
});

test("project a type's objects at the first start, then each one its own or a watched input's change touches, and none at an unchanged restart", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // project each reminder again as it changes, or as a gate naming its topic changes
    const projected: string[] = [];
    const projectedReminder = reminder.implement({
        projections: [
            {
                watches: [
                    ObjectWatch.of(gate.table, async (row, database) =>
                        (
                            await database
                                .select({ id: reminder.table.id })
                                .from(reminder.table)
                                .where(eq(reminder.table.topic, row.outcome))
                        ).map((each) => ({ id: each.id })),
                    ),
                ],
                project: async (_id, { rows }) => {
                    projected.push(...rows.map((row) => row.topic));
                },
            },
        ],
    });
    const storage = await TestDatabase.create(reminderDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([projectedReminder, gate]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: reminder.package },
    });
    const access = userContext(users.frank, spaceId, { environment: world }).access();
    const later = world.clock.now() + 86_400_000;
    const mutate = (calls: Parameters<typeof store.mutate>[1]["calls"]) =>
        store.mutate(access, { id: RequestId.create(world), calls });

    // project the existing reminder at the first start, due a day later so the type's controller leaves it
    await mutate([reminder.call("create", { spaceId, topic: "tea", dueAt: later })]);
    await settleControl(store);
    const first = projected.splice(0);

    // project a created reminder, then the reminder a gate's topic names, across restarts
    await mutate([reminder.call("create", { spaceId, topic: "cake", dueAt: later })]);
    await settleControl(store);
    const created = projected.splice(0);
    await mutate([gate.call("create", { spaceId, outcome: "tea" })]);
    await settleControl(store);
    const watched = projected.splice(0);
    await settleControl(store);
    // project tea once for the gate naming it, its controller's sync being status alone
    expect({ first, created, watched, unchanged: projected }).toEqual({
        first: ["tea"],
        created: ["cake"],
        watched: ["tea"],
        unchanged: [],
    });
});
