import { expect, onTestFinished, test } from "@destack/test";
import { reconciliation, testJournalKey } from "@destack/service/test";
import { principal, relation } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { eq } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { Condition } from "@destack/db/query";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { defineJournal, Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, expiring, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";

/** The space containing the alerts. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** A day in milliseconds. */
const DAY = 24 * 60 * 60 * 1000;

/** Alerts removed a month after reading, or half a year after arriving unread. */
const alert = defineObject({
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
        { after: { days: 180 }, from: "createdAt", where: Condition.missing("readAt") },
    ],
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: { get: method.get("read"), create: method.create("write", { fields: ["title"] }) },
});

/** Replayable alert requests. */
const journal = defineJournal("journal");

/** The database holding the alerts, their access and the journal. */
const alertDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...alert.tables],
});

test.each(TEST_DIALECTS)(
    "remove objects for good once a rule's window passes, as the system, and look again when the next one passes on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, alertDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId);
        const server = new ObjectServer({
            objects: { alert },
            database,
            context: () => ({
                subjects: [principal.user.reference("universe", "user-1")],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(journal, testJournalKey),
            audit: AuditRecorder.service(new AuditOutbox(database), {
                package: alert.package,
                service: "test",
            }),
        });
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: "user-1" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const create = async (
            title: string,
            times: { createdAt: number; readAt: number | null },
        ) => {
            const created = (await server.call(
                alert,
                "create",
                { spaceId, requestId: RequestId.create(), title },
                context,
            )) as { id: (typeof alert.table.$inferSelect)["id"] };
            await database.update(alert.table).set(times).where(eq(alert.table.id, created.id));

            return created.id;
        };

        // read long ago, unread long ago, and read yesterday
        const now = Date.now();
        await create("Read long ago", { createdAt: now - 40 * DAY, readAt: now - 31 * DAY });
        await create("Unread long ago", { createdAt: now - 181 * DAY, readAt: null });
        const kept = await create("Read yesterday", {
            createdAt: now - 200 * DAY,
            readAt: now - DAY,
        });

        // remove the two expired alerts and schedule the third
        const controller = expiring.controller(server);
        expect(Object.keys(alert.procedures).sort()).toEqual(["create", "get"]);
        const delay = await controller.reconcile(
            "expiry",
            reconciliation(AbortSignal.timeout(5000)),
        );
        const left = await database.select({ id: alert.table.id }).from(alert.table);
        expect([
            left.map((row) => row.id),
            await auditedActions(database, "system"),
            delay! > 28 * DAY && delay! <= 29 * DAY,
        ]).toEqual([[kept], ["Alert.expire", "Alert.expire"], true]);
    },
);

test("refuse expiry rules over a field the object lacks, or without any rule", () => {
    expect(
        [
            () => expiring.after(alert, [{ after: { days: 1 }, from: "seenAt" }]),
            () => expiring.after(alert, []),
        ].map((refused) => {
            try {
                refused();
            } catch (error) {
                return (error as Error).message;
            }

            return "accepted";
        }),
    ).toEqual([
        "object alert expires from unknown field seenAt",
        "object alert expires by no rule",
    ]);
});
