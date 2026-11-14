import { TestObjectStore, testIdentity } from "../src/test/index.ts";
import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { testCallKey } from "@destack/object/test";
import { TestDatabase } from "@destack/db/test";
import { settleControl, testEvents, serversOf } from "../src/test/index.ts";
import { none, principal, relation, universe } from "@destack/access";

import { eq, defineDatabase } from "@destack/db";
import { schema, Uuid } from "@destack/schema";
import { Replica, Scope, RequestId } from "@destack/sync";

import { defineObject, definePackage, defineShape, field } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { auditedActions } from "../src/test/index.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/stack/db.ts";
import objectPackage from "../src/package.ts";
import { expiring } from "../src/index.ts";
import { argument, plural, t } from "@destack/locale";

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
        title: t`${plural(argument("count"), { one: "Alert", other: "Alerts" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(),
            readAt: field.time().optional(),
        },
        access: {
            permissions: { read: relation("owner"), write: relation("owner") },
        },
        traits: [
            expiring([
                { after: { days: 30 }, from: "readAt" },
                { after: { days: 180 }, from: "createdAt", where: { readAt: { isNull: true } } },
            ]),
        ],
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
    const simulation = testSimulation();
    const world = simulation.environment("test");

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

    // remove the two expired alerts, then the third once its window passes
    expect(Object.keys(alert.procedures).toSorted()).toEqual([
        "attach",
        "create",
        "delete",
        "detach",
        "get",
        "list",
        "update",
    ]);
    const left = async () =>
        (await database.select({ id: alert.table.id }).from(alert.table)).map((row) => row.id);
    await settleControl(store);
    const first = await left();
    const expired = await auditedActions(database, "component");
    await simulation.jump(now + 28 * DAY);
    await settleControl(store);
    const early = await left();
    await simulation.jump(now + 29 * DAY);
    await settleControl(store);
    expect([first, expired, early, await left()]).toEqual([
        [recent],
        ["alert.expire", "alert.expire"],
        [recent],
        [],
    ]);
});

test("leave the expired rows a database replicates to the deletes their home sends", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // follow the alerts, replicating their table
    const follow = defineShape({
        name: "alerts",
        parameters: schema.object({}),
        objects: [alert],
        queries: () => [alert.query()],
    }).onDemand({ minutes: 10 });
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
    await settleControl(store);
    const left = await database.select({ title: alert.table.title }).from(alert.table);
    expect(left).toEqual([{ title: "Replicated" }]);
});

test("grant expiry of a space type and a universe type to no one, leaving it to the installations implementing them", () => {
    // declare one expiring type in spaces and one in the universe alone, neither declaring expire
    const universal = defineObject({
        name: "token",
        plural: "tokens",
        title: t`${plural(argument("count"), { one: "Token", other: "Tokens" })}`,
        scope: Scope.universe.id,
        fields: { expiresAt: field.time() },
        access: {
            permissions: ["read"],
        },
        traits: [expiring([{ after: { days: 1 }, from: "expiresAt" }])],
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
        title: t`${plural(argument("count"), { one: "Person", other: "People" })}`,
        scope: Scope.universe.id,
        fields: {},
        container: "scope",
        access: {
            permissions: { read: none() },
        },
    });
    const reminder = defineObject({
        name: "reminder",
        plural: "reminders",
        title: t`${plural(argument("count"), { one: "Reminder", other: "Reminders" })}`,
        scope: person,
        fields: { expiresAt: field.time() },
        access: {
            permissions: ["read"],
        },
        traits: [expiring([{ after: { milliseconds: 0 }, from: "expiresAt" }])],
    });
    await using objects = await TestObjectStore.open({
        packages: [
            definePackage({
                ...testIdentity,
                objects: [person, reminder],
            }),
        ],
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

    // let the store's expiry remove both, each in its own scope's mutation, failing the world otherwise
    await objects.simulation.idle();
    expect(await objects.database.select().from(reminder.table)).toEqual([]);
});
