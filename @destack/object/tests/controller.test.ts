import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testCallKey } from "@destack/object/test";
import { reconciliation, TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { AccessContext, Caller, none, principal, relation } from "@destack/access";

import { Scope, RequestId } from "@destack/sync";
import { asc, eq, defineDatabase, type Change, Snapshot } from "@destack/db";
import { schema, present, type Identifier } from "@destack/schema";

import { defineObject, field, ObjectWatch } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { auditedCalls } from "./fixture/audit.ts";
import { Clock } from "@destack/environment";

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
        send: method.mutation({ permission: "send", isInternal: true }),
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

/** The database with the reminders, their access and the journal. */
const reminderDatabase = defineDatabase({
    name: "main",
    tables: [...callTables, ...reminder.tables],
});

test.each(TEST_DIALECTS)(
    "reconcile a type's pending objects by the key its controller declares, as the store's principal, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, reminderDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([reminder]),
            database,
            callKey: testCallKey,
            events: await testEvents(database),
            origin: {
                package: reminder.package,
            },
        });
        const controller = present(
            store.controllers().find((each) => each.name === "reminder"),
            "the reminder controller",
        );

        // insert two due reminders about tea, one later about tea, and one sent about lunch
        const context = userContext(users.frank, spaceId);
        const now = Date.now();
        const create = async (topic: string, dueAt: number) =>
            store
                .mutate(context.access(), {
                    id: RequestId.create(Date.now()),
                    calls: [reminder.call("create", { spaceId, topic, dueAt })],
                })
                .then(([written]) => written);
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
            .orderBy(asc(reminder.table.id));
        expect([sent.map((each) => each.sentAt !== null), wait > 50_000 && wait <= 60_000]).toEqual(
            [[true, true, false], true],
        );
    },
);

test("select a changed pending object's own key beside the keys every watch of its own table declares", async () => {
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
    const storage = await TestDatabase.create("sqlite", reminderDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([queued]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: reminder.package },
    });
    const controller = present(
        store.controllers().find((each) => each.name === "reminder"),
        "the reminder controller",
    );
    await openSpace(storage.database, spaceId);
    const created = await store
        .mutate(userContext(users.frank, spaceId).access(), {
            id: RequestId.create(Date.now()),
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

test("run a controller's calls as the space, audited as the space with the controller's name, and refuse principals lacking the method's permission", async () => {
    const storage = await TestDatabase.create("sqlite", reminderDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([reminder]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: reminder.package },
    });
    const controller = present(
        store.controllers().find((each) => each.name === "reminder"),
        "the reminder controller",
    );

    // send a due reminder of frank through the controller
    const due = await store
        .mutate(userContext(users.frank, spaceId).access(), {
            id: RequestId.create(Date.now()),
            calls: [reminder.call("create", { spaceId, topic: "tea", dueAt: Date.now() - 1000 })],
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
                    Date.now(),
                ),
                {
                    id: RequestId.create(Date.now()),
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
        ["FORBIDDEN", "permission denied: send"],
        ["NOT_FOUND", `no reminder ${due.id}`],
    ]);
});
