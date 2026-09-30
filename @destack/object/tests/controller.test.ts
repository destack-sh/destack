import { expect, onTestFinished, test } from "@destack/test";
import { reconciliation } from "@destack/service/test";
import { principal, relation } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { asc, eq } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { Condition } from "@destack/db/query";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { defineJournal, Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";

/** The space holding the reminders. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

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
    methods: {
        create: method.create("write", { fields: ["topic", "dueAt"] }),
        send: method({ permission: null, isSystem: true }).handle((call) =>
            call.revise({ sentAt: call.now }),
        ),
    },
    controller: {
        pending: Condition.missing("sentAt"),
        key: (row) => ({ topic: row.topic }),
        async reconcile({ rows, now, execute }) {
            // send the due reminders of the topic together, and wait for the next one
            const due = rows.filter((row) => Number(row.dueAt) <= now);
            if (due.length > 0) {
                await execute("send", due);
            }
            const next = rows
                .filter((row) => Number(row.dueAt) > now)
                .map((row) => Number(row.dueAt) - now);

            return next.length === 0 ? undefined : Math.min(...next);
        },
    },
});

/** Replayable reminder requests. */
const journal = defineJournal("journal");

/** The database holding the reminders, their access and the journal. */
const reminderDatabase = defineDatabase({ name: "main", tables: [journal, ...reminder.tables] });

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
            context: () => ({
                subjects: [principal.user.reference("universe", "user-1")],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(journal),
            audit: AuditRecorder.service(new AuditOutbox(database), {
                package: reminder.package,
                service: "test",
            }),
        });
        const controller = server.controllers().find((each) => each.name === "reminder")!;

        // hold two due reminders about tea, one later about tea, and one sent about lunch
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: "user-1" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const now = Date.now();
        const create = async (topic: string, dueAt: number) =>
            (await server.call(
                reminder,
                "create",
                { spaceId, requestId: RequestId.create(), topic, dueAt },
                context,
            )) as typeof reminder.table.$inferSelect;
        await create("tea", now - 1000);
        await create("tea", now - 500);
        await create("tea", now + 60_000);
        const lunch = await create("lunch", now - 1000);
        await database
            .update(reminder.table)
            .set({ sentAt: now - 900 })
            .where(eq(reminder.table.id, lunch.id));
        const row = (topic: string, sentAt: number | null) => ({ ...lunch, topic, sentAt });

        // list one key per topic with work waiting, and key a changed pending object by its topic
        const keys = await controller.list();
        expect([
            keys,
            await controller.keys!({
                table: reminder.table,
                after: row("cake", null),
            } as never),
            await controller.keys!({ table: reminder.table, after: row("cake", now) } as never),
        ]).toEqual([['{"topic":"tea"}'], ['{"topic":"cake"}'], []]);

        // send the due tea reminders together and look again when the last one is due
        const wait = await controller.reconcile(
            keys[0]!,
            reconciliation(AbortSignal.timeout(5000)),
        );
        const sent = await database
            .select({ id: reminder.table.id, sentAt: reminder.table.sentAt })
            .from(reminder.table)
            .where(eq(reminder.table.topic, "tea"))
            .orderBy(asc(reminder.table.id));
        expect([
            sent.map((each) => each.sentAt !== null),
            wait! > 50_000 && wait! <= 60_000,
        ]).toEqual([[true, true, false], true]);
    },
);

test("refuse a controller on ephemeral objects", () => {
    expect(() =>
        defineObject({
            name: "cursor",
            plural: "cursors",
            scope: space,
            storage: "ephemeral",
            fields: { position: field.integer() },
            permissions: { read: relation("owner") },
            controller: { pending: Condition.all(), reconcile: async () => undefined },
        } as never),
    ).toThrow(new TypeError("ephemeral object cursor takes no controller"));
});
