import { expect, onTestFinished, test } from "@destack/test";
import { reconciliation, testCallKey } from "@destack/service/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit";
import { asc, eq, defineDatabase, type Change } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, present } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { defineObject, field, ObjectWatch } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** The space with the reminders. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Reminders the system sends once due, a batch per topic. */
const reminder = defineObject({
    name: "reminder",
    plural: "reminders",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        topic: field.string(),
        dueAt: field.time(),
        sentAt: field.time().optional(),
    },
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: (method) => ({
        create: method.create("write", { fields: ["topic", "dueAt"] }),
        send: method
            .mutation({ permission: null, isSystem: true })
            .handle((call) => call.update({ sentAt: call.now })),
    }),
}).control({
    pending: { sentAt: { isNull: true } },
    key: (row) => ({ topic: row.topic }),
    async reconcile(reconciling) {
        // send the due reminders of the topic together, and wait for the next one
        const { rows, now } = reconciling;
        const due = rows.filter((row) => row.dueAt <= now);
        if (due.length > 0) {
            await reconciling.execute("send", due);
        }
        const next = rows.filter((row) => row.dueAt > now).map((row) => row.dueAt - now);

        return next.length === 0 ? undefined : Math.min(...next);
    },
});

/** The database with the reminders, their access and the journal. */
const reminderDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...reminder.tables],
});

test.each(TEST_DIALECTS)(
    "reconcile a type's pending objects by the key its controller declares, as the system, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, reminderDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId);
        const server = new ObjectServer({
            objects: { reminder },
            database,
            callKey: testCallKey,
            origin: {
                package: reminder.package,
                service: "test",
            },
        });
        const controller = present(
            server.controllers().find((each) => each.name === "reminder"),
            "the reminder controller",
        );

        // insert two due reminders about tea, one later about tea, and one sent about lunch
        const context = userContext("user-1", spaceId);
        const now = Date.now();
        const create = async (topic: string, dueAt: number) =>
            server.call(
                reminder,
                "create",
                { spaceId, requestId: RequestId.create(), topic, dueAt },
                context,
            );
        await create("tea", now - 1000);
        await create("tea", now - 500);
        await create("tea", now + 60_000);
        const lunch = await create("lunch", now - 1000);
        await database
            .update(reminder.table)
            .set({ sentAt: now - 900 })
            .where(eq(reminder.table.id, lunch.id));
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
        const keys = await controller.list();
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
                reconciliation(AbortSignal.timeout(5000)),
            ),
            "the wait until the next reminder",
        );
        const sent = await database
            .select({ id: reminder.table.id, sentAt: reminder.table.sentAt })
            .from(reminder.table)
            .where(eq(reminder.table.topic, "tea"))
            .orderBy(asc(reminder.table.id));
        expect([sent.map((each) => each.sentAt !== null), wait > 50_000 && wait <= 60_000]).toEqual(
            [[true, true, false], true],
        );
    },
);

test("select a changed pending object's own key beside the keys a watch of its own table declares", async () => {
    // wake the earliest unsent reminder of a topic once another of its reminders changes
    const queued = reminder.control({
        pending: { sentAt: { isNull: true } },
        watches: [ObjectWatch.of(reminder.table, () => [{ id: "head" }])],
        reconcile: async () => undefined,
    });
    const storage = await TestDatabase.create("sqlite", reminderDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const server = new ObjectServer({
        objects: { reminder: queued },
        database: storage.database,
        callKey: testCallKey,
        origin: { package: reminder.package, service: "test" },
    });
    const controller = present(
        server.controllers().find((each) => each.name === "reminder"),
        "the reminder controller",
    );
    await openSpace(storage.database, spaceId);
    const created = await server.call(
        reminder,
        "create",
        { spaceId, requestId: RequestId.create(), topic: "tea", dueAt: 1 },
        userContext("user-1", spaceId),
    );
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

    // key a pending reminder by itself and the watch, and a sent one by the watch alone
    expect([
        await controller.keys?.(changed(null)),
        await controller.keys?.(changed(1)),
        controller.watches,
    ]).toEqual([['{"id":"head"}', `{"id":"${created.id}"}`], ['{"id":"head"}'], [reminder.table]]);
});

test("refuse a controller on ephemeral objects", () => {
    expect(() =>
        defineObject({
            name: "cursor",
            plural: "cursors",
            scope: space,
            storage: "ephemeral",
            fields: { position: field.integer() },
            permissions: { read: relation("owner") },
        }).control({ pending: {}, reconcile: async () => undefined }),
    ).toThrow(new TypeError("ephemeral object cursor takes no controller"));
});
