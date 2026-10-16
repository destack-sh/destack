import { expect, onTestFinished, test } from "@destack/test";
import { reconciliation, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { principal, relation } from "@destack/access";

import { eq, defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { v7 } from "uuid";

import { RequestId } from "@destack/runtime/request";
import { defineObject, field } from "../src/index.ts";
import { ExpiringController, ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext, users } from "./fixture/user.ts";
import { Clock } from "@destack/environment";

/** The space containing the alerts. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

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
    tables: [...callTables, ...alert.tables],
});

test.each(TEST_DIALECTS)(
    "remove objects for good once a rule's window passes, as the system, and look again when the next one passes on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, alertDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([alert]),
            database,
            callKey: testCallKey,
            events: await testEvents(database),
            origin: {
                package: alert.package,
            },
        });
        const context = userContext(users.frank, spaceId);
        const create = async (
            title: string,
            times: { createdAt: number; readAt: number | null },
        ) => {
            const created = await store
                .mutate(context.access(), {
                    id: RequestId.create(Date.now()),
                    calls: [alert.call("create", { spaceId, title })],
                })
                .then(([written]) => written);
            await database.update(alert.table).set(times).where(eq(alert.table.id, created.id));

            return created.id;
        };

        // read long ago, unread long ago, and read yesterday
        const now = Date.now();
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
    },
);

test("leave the expired rows a database replicates to the deletes their home sends", async () => {
    // store the alerts as replicas, one long expired
    const replicated = defineDatabase({
        name: "main",
        tables: [...callTables, ...alert.tables.filter((table) => table !== alert.table)],
        replicated: [alert.table],
    });
    const storage = await TestDatabase.create("sqlite", replicated, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([alert]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: alert.package },
    });
    const now = Date.now();
    await database.insert(alert.table).values({
        id: alert.identifier(`alert-${v7()}`),
        scope: spaceId,
        ownerId: users.frank,
        title: "Replicated",
        createdAt: now - 400 * DAY,
        updatedAt: now - 400 * DAY,
        readAt: null,
    });

    // expire nothing and schedule nothing
    const controller = new ExpiringController(store);
    const delay = await controller.reconcile(
        "expiry",
        reconciliation(new AbortController().signal),
    );
    const left = await database.select({ title: alert.table.title }).from(alert.table);
    expect([left, delay, controller.watches]).toEqual([[{ title: "Replicated" }], undefined, []]);
});
