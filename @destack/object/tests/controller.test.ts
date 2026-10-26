import { TestObjectStore } from "../src/test/index.ts";
import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testCallKey } from "@destack/object/test";
import { reconciliation, TestDatabase } from "@destack/db/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { AccessContext, Caller, InstallationId, none, principal, relation } from "@destack/access";

import { Scope, RequestId } from "@destack/sync";
import {
    asc,
    eq,
    defineDatabase,
    type Change,
    Snapshot,
    type ReconcileController,
} from "@destack/db";
import { Failure } from "@destack/error";
import { schema, present, type Identifier } from "@destack/schema";

import { defineObject, field, ObjectWatch } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { auditedCalls } from "./fixture/audit.ts";
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
    controller: {
        pending: { sentAt: { isNull: true } },
        key: (row) => ({ topic: row.topic }),
        async reconcile(reconciling) {
            // send the due reminders of the topic together, and wait for the next one
            const { rows, now } = reconciling;
            const due = rows.filter((row) => row.dueAt <= now);
            if (due.length > 0) {
                await reconciling.mutate(
                    due.map((row) => reminder.call("send", reminder.target(row))),
                );
            }
            const next = rows.filter((row) => row.dueAt > now).map((row) => row.dueAt - now);

            return next.length === 0 ? undefined : Math.min(...next);
        },
    },
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
    controller: {
        pending: {},
        async reconcile(reconciling) {
            // refuse or fail each gate as its outcome says
            for (const row of reconciling.rows) {
                if (row.outcome === "refused") {
                    throw Failure.error({
                        code: "FORBIDDEN",
                        reason: "PERMISSION_DENIED",
                        domain: "@destack/access",
                        message: "permission denied: bind",
                    });
                } else if (row.outcome === "unavailable") {
                    throw Failure.error(Failure.http("SERVICE_UNAVAILABLE", "service unavailable"));
                }
            }

            return undefined;
        },
    },
});

/** The database with the reminders, their access and the journal. */
const reminderDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { reminder, gate }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

test("reconcile a type's pending objects by the key its controller declares, as the store's principal", async () => {
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
        origin: {
            package: reminder.package,
        },
    });
    const controller = present(
        store
            .controllers()
            .find(
                (each): each is ReconcileController =>
                    each.name === "reminder" && each.mode !== "follow",
            ),
        "the reminder controller",
    );

    // insert two due reminders about tea, one later about tea, and one sent about lunch
    const context = userContext(users.frank, spaceId, { environment: world });
    const now = world.clock.now();
    const create = async (topic: string, dueAt: number) =>
        store
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [reminder.call("create", { spaceId, topic, dueAt })],
            })
            .then(([written]) => written);
    await create("tea", now - 1000);
    await create("tea", now - 500);
    await create("tea", now + 60_000);
    const lunch = await create("lunch", now - 1000);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(reminder.table)
                .set({ sentAt: now - 900 })
                .where(eq(reminder.table.id, lunch.id)),
    );
    const inserted = (topic: string, sentAt: number | null): Change<typeof reminder.table> => ({
        transaction: null,
        table: reminder.table,
        key: { id: lunch.id },
        scope: spaceId,
        changedAt: now,
        operation: "insert",
        after: { ...lunch, topic, sentAt },
        sequence: 1,
    });

    // list one key per topic with work waiting, and key a changed pending object by its topic
    const keys = await controller.list(Snapshot.live(database));
    if (controller.keys === undefined) {
        throw new TypeError("the reminder controller keys no changes");
    }
    expect([
        keys,
        await controller.keys(inserted("cake", null)),
        await controller.keys(inserted("cake", now)),
    ]).toEqual([['{"topic":"tea"}'], ['{"topic":"cake"}'], []]);

    // send the due tea reminders together and look again when the last one is due
    const wait = present(
        await controller.reconcile(
            present(keys[0], "the tea key"),
            reconciliation(new AbortController().signal),
        ),
        "the wait until the next reminder",
    );
    const sent = await database
        .select({ id: reminder.table.id, sentAt: reminder.table.sentAt })
        .from(reminder.table)
        .where(eq(reminder.table.topic, "tea"))
        .orderBy(asc(reminder.table.dueAt));
    expect([sent.map((each) => each.sentAt !== null), wait > 50_000 && wait <= 60_000]).toEqual([
        [true, true, false],
        true,
    ]);
});

test("select a changed pending object's key beside the keys every watch of its table declares", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // wake the first and the last unsent reminder through two watches once another reminder changes
    const queued = reminder.implement({
        controller: {
            pending: { sentAt: { isNull: true } },
            watches: [
                ObjectWatch.of(reminder.table, () => [{ id: "head" }]),
                ObjectWatch.of(reminder.table, () => [{ id: "tail" }]),
            ],
            reconcile: async () => undefined,
        },
    });
    const storage = await TestDatabase.create(reminderDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([queued]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: reminder.package },
    });
    const controller = present(
        store
            .controllers()
            .find(
                (each): each is ReconcileController =>
                    each.name === "reminder" && each.mode !== "follow",
            ),
        "the reminder controller",
    );
    const created = await store
        .mutate(userContext(users.frank, spaceId, { environment: world }).access(), {
            id: RequestId.create(world),
            calls: [reminder.call("create", { spaceId, topic: "tea", dueAt: 1 })],
        })
        .then(([written]) => written);
    const changed = (sentAt: number | null): Change<typeof reminder.table> => ({
        transaction: null,
        table: reminder.table,
        key: { id: created.id },
        scope: spaceId,
        changedAt: 1,
        operation: "insert",
        after: { ...created, sentAt },
        sequence: 1,
    });

    // key a pending reminder by itself and both watches, and a sent one only by the watches
    expect([
        await controller.keys?.(changed(null)),
        await controller.keys?.(changed(1)),
        controller.watches,
    ]).toEqual([
        ['{"id":"head"}', '{"id":"tail"}', `{"id":"${created.id}"}`],
        ['{"id":"head"}', '{"id":"tail"}'],
        [reminder.table],
    ]);
});

test("run a controller's calls as its package's base installation, audited with the controller's name, and refuse principals lacking the method's permission", async () => {
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
    const controller = present(
        store
            .controllers()
            .find(
                (each): each is ReconcileController =>
                    each.name === "reminder" && each.mode !== "follow",
            ),
        "the reminder controller",
    );

    // send a due reminder of frank through the controller
    const due = await store
        .mutate(userContext(users.frank, spaceId, { environment: world }).access(), {
            id: RequestId.create(world),
            calls: [
                reminder.call("create", { spaceId, topic: "tea", dueAt: world.clock.now() - 1000 }),
            ],
        })
        .then(([written]) => written);
    await controller.reconcile('{"topic":"tea"}', reconciliation(new AbortController().signal));
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

    // record the send as the reminder package's base installation and the controller, and refuse both users
    const installationId = await InstallationId.base(
        space.reference(Scope.universe.id, spaceId),
        reminder.package.id,
        world.crypto,
    );
    expect([
        sends.map((call) => [call.execution.context.caller, call.execution.context.component]),
        await send(users.frank),
        await send(users.grace),
    ]).toEqual([
        [[{ subject: principal.installation.reference(spaceId, installationId) }, "reminder"]],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: send"],
        ["NOT_FOUND", "NOT_FOUND", `reminder ${due.id} not found`],
    ]);
});

test("record a lasting reconcile failure on a controlled object as its synced condition, and clear it once reconciled", async () => {
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
    const controller = present(
        store
            .controllers()
            .find(
                (each): each is ReconcileController =>
                    each.name === "gate" && each.mode !== "follow",
            ),
        "the gate controller",
    );

    // create a refused gate and an unavailable one
    const access = userContext(users.frank, spaceId, { environment: world }).access();
    const create = async (outcome: string) =>
        store
            .mutate(access, {
                id: RequestId.create(world),
                calls: [gate.call("create", { spaceId, outcome })],
            })
            .then(([written]) => written);
    const refused = await create("refused");
    const unavailable = await create("unavailable");

    // reconcile each, then open the refused gate and reconcile it again
    const reconcile = async (id: string) =>
        refusal(
            controller.reconcile(
                JSON.stringify({ id }),
                reconciliation(new AbortController().signal),
            ),
        );
    const failed = [await reconcile(refused.id), await reconcile(unavailable.id)];
    const conditions = async () =>
        Object.fromEntries(
            (await database.select().from(gate.table).orderBy(asc(gate.table.outcome))).map(
                (row) => [row.outcome, row.conditions],
            ),
        );
    const recorded = await conditions();
    const read = async () =>
        present(
            (await database.select().from(gate.table)).find((row) => row.id === refused.id),
            "the refused gate",
        );
    const observed = await read();
    await store.mutate(access, {
        id: RequestId.create(world),
        calls: [gate.call("update", { ...gate.target(refused), outcome: "open" })],
    });
    const opened = await read();

    // key the opening, and skip the failure's own write, which left the generation as it was
    const updated = (
        before: typeof observed,
        after: typeof observed,
    ): Change<typeof gate.table> => ({
        transaction: null,
        table: gate.table,
        key: { id: refused.id },
        scope: spaceId,
        changedAt: world.clock.now(),
        operation: "update",
        before,
        after,
        sequence: 1,
    });
    const keyed = [
        await controller.keys?.(updated(refused, observed)),
        await controller.keys?.(updated(observed, opened)),
    ];
    await controller.reconcile(
        JSON.stringify({ id: refused.id }),
        reconciliation(new AbortController().signal),
    );

    // record the refusal with its domain and reason, leave the outage unrecorded, and clear the refusal once open
    const now = world.clock.now();
    expect({ failed, recorded, keyed, cleared: await conditions() }).toEqual({
        keyed: [[], [JSON.stringify({ id: refused.id })]],
        failed: [
            ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: bind"],
            ["SERVICE_UNAVAILABLE", "SERVICE_UNAVAILABLE", "service unavailable"],
        ],
        recorded: {
            refused: {
                Synced: {
                    status: "false",
                    observedGeneration: 1,
                    reason: "ReconcileError",
                    message: "@destack/access PERMISSION_DENIED: permission denied: bind",
                    lastTransitionAt: now,
                },
            },
            unavailable: {},
        },
        cleared: {
            open: {
                Synced: {
                    status: "true",
                    observedGeneration: 2,
                    reason: "ReconcileSuccess",
                    message: "",
                    lastTransitionAt: now,
                },
            },
            unavailable: {},
        },
    });
});
