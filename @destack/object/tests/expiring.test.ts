import { TestObjectStore } from "../src/test/index.ts";
import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { testCallKey } from "@destack/object/test";
import { reconciliation, TestDatabase } from "@destack/db/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { none, principal, relation, universe } from "@destack/access";

import { eq, defineDatabase } from "@destack/db";
import { schema, Uuid } from "@destack/schema";
import { Replica, Scope, RequestId } from "@destack/sync";

import { defineObject, definePackage, defineShape, field, Follow } from "../src/index.ts";
import { ExpiringController } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/stack/db.ts";
import objectPackage from "../src/package.ts";

/** The space containing the alerts. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** The two people whose reminders expire at once. */
const PEOPLE = [
    "person-01996ab0-0000-7000-8000-000000000003",
    "person-01996ab0-0000-7000-8000-000000000004",
];

/** A day in milliseconds. */
const DAY = 24 * 60 * 60 * 1000;

/** Alerts removed a month after reading, or half a year after arriving unread. */
const alert = defineObject(
    {
        name: "alert",
        plural: "alerts",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(),
            readAt: field.time().optional(),
        },
        expiring: [
            { after: { days: 30 }, from: "readAt" },
            { after: { days: 180 }, from: "createdAt", where: { readAt: { isNull: true } } },
        ],
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", { fields: ["title"] }),
    }),
);

/** The database with the alerts, their access and the journal. */
const alertDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { alert }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

test("remove objects for good once a rule's window passes, as the system, and look again when the next one passes", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(alertDatabase, {
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
        servers: serversOf([alert]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: {
            package: alert.package,
        },
    });
    const context = userContext(users.frank, spaceId, { environment: world });
    const create = async (title: string, times: { createdAt: number; readAt: number | null }) => {
        const created = await store
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [alert.call("create", { spaceId, title })],
            })
            .then(([written]) => written);
        await database.transaction(
            async (transaction) =>
                await transaction
                    .update(alert.table)
                    .set(times)
                    .where(eq(alert.table.id, created.id)),
        );

        return created.id;
    };

    // read long ago, unread long ago, and read yesterday
    const now = world.clock.now();
    await create("Read long ago", { createdAt: now - 40 * DAY, readAt: now - 31 * DAY });
    await create("Unread long ago", { createdAt: now - 181 * DAY, readAt: null });
    const recent = await create("Read yesterday", {
        createdAt: now - 200 * DAY,
        readAt: now - DAY,
    });

    // remove the two expired alerts and schedule the third
    const controller = new ExpiringController(store);
    expect(Object.keys(alert.procedures).toSorted()).toEqual([
        "create",
        "delete",
        "get",
        "list",
        "update",
    ]);
    const delay = await controller.reconcile(
        "expiry",
        reconciliation(new AbortController().signal),
    );
    const left = await database.select({ id: alert.table.id }).from(alert.table);
    expect([
        left.map((row) => row.id),
        await auditedActions(database, "component"),
        delay !== undefined && delay > 28 * DAY && delay <= 29 * DAY,
    ]).toEqual([[recent], ["alert.expire", "alert.expire"], true]);
});

test("leave the expired rows a database replicates to the deletes their home sends", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // follow the alerts, replicating their table
    const follow = Follow.of(
        defineShape({
            name: "alerts",
            parameters: schema.object({}),
            objects: [alert],
            queries: () => [alert.query()],
        }),
        { onDemand: { ttl: { minutes: 10 } } },
    );
    const replicated = defineDatabase({
        name: "main",
        ...tablesOf({ packages: [objectPackage], follows: [follow], tables: [] }),
    });
    const storage = await TestDatabase.create(replicated, {
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
        servers: serversOf([alert], { follows: [follow] }),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: alert.package },
    });
    // replicate another space's alerts, and keep a long expired alert in each space
    const other = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000005");
    await new Replica({ name: "alerts", scope: other, tables: [alert.table] }).register(database);
    const now = world.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(alert.table).values(
                (
                    [
                        [spaceId, "Owned"],
                        [other, "Replicated"],
                    ] as const
                ).map(([scope, title]) => ({
                    id: alert.identifier(`alert-${Uuid.v7(world)}`),
                    scope,
                    ownerId: users.frank,
                    title,
                    createdAt: now - 400 * DAY,
                    updatedAt: now - 400 * DAY,
                    readAt: null,
                })),
            ),
    );

    // expire the owned alert alone, leaving the replicated one to its home
    const controller = new ExpiringController(store);
    const delay = await controller.reconcile(
        "expiry",
        reconciliation(new AbortController().signal),
    );
    const left = await database.select({ title: alert.table.title }).from(alert.table);
    expect([left, delay, controller.watches]).toEqual([
        [{ title: "Replicated" }],
        undefined,
        [alert.table],
    ]);
});

test("grant expiry of a space type and a universe type to no one, leaving it to the installations implementing them", () => {
    // declare one expiring type in spaces and one in the universe alone, neither declaring expire
    const universal = defineObject({
        name: "token",
        plural: "tokens",
        scope: Scope.universe.id,
        fields: { expiresAt: field.time() },
        expiring: [{ after: { days: 1 }, from: "expiresAt" }],
        permissions: ["read"],
    });

    // expect both types' expiry granted to no one
    expect([
        alert.policy.definition.permissions["expire"],
        universal.policy.definition.permissions["expire"],
    ]).toEqual([none(), none()]);
});

test("remove the expired objects of several people at once, one mutation per scope", async () => {
    // serve reminders in two people's scopes at the universe, one expired in each
    const person = defineObject({
        name: "person",
        plural: "people",
        scope: Scope.universe.id,
        isScope: true,
        fields: {},
        permissions: { read: none() },
    });
    const reminder = defineObject({
        name: "reminder",
        plural: "reminders",
        scope: person,
        fields: { expiresAt: field.time() },
        expiring: [{ after: { milliseconds: 0 }, from: "expiresAt" }],
        permissions: ["read"],
    });
    await using objects = await TestObjectStore.open({
        packages: [definePackage({ objects: { person, reminder } })],
        scope: universe.reference(Scope.universe.id, Scope.universe.id),
        scopes: PEOPLE.map((id) => person.reference(Scope.universe.id, id)),
    });
    const now = objects.environment.clock.now();
    await objects.database.transaction(
        async (transaction) =>
            await transaction.insert(reminder.table).values(
                PEOPLE.map((id) => ({
                    id: reminder.identifier(`reminder-${Uuid.v7(objects.environment)}`),
                    scope: person.identifier(id),
                    createdAt: now - DAY,
                    updatedAt: now - DAY,
                    expiresAt: now - 1,
                })),
            ),
    );

    // let the store's expiry remove both, each in its own scope's mutation, failing the settling otherwise
    await objects.settle();
    expect(await objects.database.select().from(reminder.table)).toEqual([]);
});
