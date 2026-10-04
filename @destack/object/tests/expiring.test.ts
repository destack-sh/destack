import { expect, onTestFinished, test } from "@destack/test";
import { reconciliation, testCallKey } from "@destack/service/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit";
import { eq, defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { defineObject, expiring, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext } from "./fixture/user.ts";

/** The space containing the alerts. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

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
        { after: { days: 180 }, from: "createdAt", where: { readAt: { isNull: true } } },
    ],
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: (method) => ({
        get: method.get("read"),
        create: method.create("write", { fields: ["title"] }),
    }),
});

/** The database with the alerts, their access and the journal. */
const alertDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...alert.tables],
    copies: [],
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
            callKey: testCallKey,
            origin: {
                package: alert.package,
                service: "test",
            },
        });
        const context = userContext("user-1", spaceId);
        const create = async (
            title: string,
            times: { createdAt: number; readAt: number | null },
        ) => {
            const created = await server.call(
                alert,
                "create",
                { spaceId, requestId: RequestId.create(), title },
                context,
            );
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
        expect(Object.keys(alert.procedures).toSorted()).toEqual(["create", "get"]);
        const delay = await controller.reconcile(
            "expiry",
            reconciliation(AbortSignal.timeout(5000)),
        );
        const left = await database.select({ id: alert.table.id }).from(alert.table);
        expect([
            left.map((row) => row.id),
            await auditedActions(database, "system"),
            delay !== undefined && delay > 28 * DAY && delay <= 29 * DAY,
        ]).toEqual([[kept], ["alert.expire", "alert.expire"], true]);
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
                if (!(error instanceof Error)) {
                    throw error;
                }

                return error.message;
            }

            return "accepted";
        }),
    ).toEqual([
        "object alert expires from unknown field seenAt",
        "object alert expires by no rule",
    ]);
});
