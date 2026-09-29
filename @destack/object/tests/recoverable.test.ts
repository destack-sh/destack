import type { Change } from "@destack/db/log";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { eq } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { defineJournal, Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, field, method, type ObjectType } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { recoverable } from "../src/trait/recoverable.ts";
import { openSpace, space } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";

/** The space containing the credentials. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** Credentials that keep their record after a purge, reserved to their custodian. */
const credential = defineObject({
    name: "credential",
    plural: "credentials",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        custodian: field.reference(principal.user),
        value: field.string().optional(),
    },
    recoverable: { within: { minutes: 1 }, by: "write", purge: "purge", keep: "record" },
    permissions: {
        read: relation("owner"),
        write: relation("owner"),
        purge: relation("custodian"),
    },
    methods: {
        get: method.get("read"),
        create: method.create("write", { fields: ["custodian", "value"] }),
    },
});

/** Replayable credential requests. */
const journal = defineJournal("journal");

/** The database holding the credentials, their access and the journal. */
const credentialDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...credential.tables],
});

test.each(TEST_DIALECTS)(
    "purge a kept record through its handler under its own permission or as the system, and restore within the host's window on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, credentialDatabase, {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId);

        // serve credentials restorable for a week with a purge handler that destroys the value
        const handled = recoverable.within(
            credential.handle({
                purge: async (call, next) => {
                    await call.database
                        .update(credential.table)
                        .set({ value: null })
                        .where(eq(credential.table.id, call.target!.id));

                    return next();
                },
            }),
            { days: 7 },
        );
        let current = "user-1";
        const serve = (object: ObjectType) =>
            new ObjectServer({
                objects: { credential: object },
                database,
                context: () => ({
                    subjects: [principal.user.reference("universe", current)],
                    now: Date.now(),
                    attributes: {},
                }),
                journal: new Journal(journal),
                audit: AuditRecorder.service(new AuditOutbox(database), {
                    package: credential.package,
                    service: "test",
                }),
            });
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: current }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const execute = (object: ObjectType, name: string, input: Record<string, unknown>) =>
            serve(object).call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            ) as Promise<typeof credential.table.$inferSelect>;
        // restore within the host's window what the declared window no longer restores
        const created = await execute(handled, "create", {
            custodian: "user-2",
            value: "hunter2",
        });
        await execute(handled, "delete", { id: created.id, revision: created.revision });
        await database
            .update(credential.table)
            .set({ deletionRequestedAt: Date.now() - 120_000 })
            .where(eq(credential.table.id, created.id));
        await expect(execute(credential, "restore", { id: created.id })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "credential is past its recovery window",
        });
        const restored = await execute(handled, "restore", { id: created.id });
        expect(restored.deletionRequestedAt).toBeNull();

        // reserve purging to the custodian, even for the owner deleting and restoring
        await execute(handled, "delete", { id: created.id, revision: restored.revision });
        await expect(execute(handled, "purge", { id: created.id })).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: purge",
        });

        // refuse a record-keeping purge without a handler destroying the content
        current = "user-2";
        await expect(execute(credential, "purge", { id: created.id })).rejects.toThrow(
            new TypeError(
                "object credential keeps purged records, so a purge handler of its own destroys their content",
            ),
        );

        // destroy the value and keep the record, marked purged
        await execute(handled, "purge", { id: created.id });
        const [purged] = await database
            .select()
            .from(credential.table)
            .where(eq(credential.table.id, created.id));
        expect(purged).toEqual({
            ...created,
            value: null,
            revision: 5,
            updatedAt: expect.any(Number),
            deletionRequestedAt: expect.any(Number),
            purgedAt: expect.any(Number),
        });

        // refuse purging and restoring a purged record
        await expect(execute(handled, "purge", { id: created.id })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "credential is purged",
        });
        current = "user-1";
        await expect(execute(handled, "restore", { id: created.id })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "credential is purged",
        });

        // purge a record past the host's window as the system, through its handler, once
        const expired = await execute(handled, "create", {
            custodian: "user-2",
            value: "swordfish",
        });
        await execute(handled, "delete", { id: expired.id, revision: expired.revision });
        const server = serve(handled);
        const later = Date.now() + 8 * 24 * 60 * 60 * 1000;
        expect([
            await recoverable.purge(server, later),
            await recoverable.purge(server, later),
        ]).toEqual([1, 0]);
        expect(await auditedActions(database, "system")).toEqual(["Credential.purge"]);
        const [kept] = await database
            .select({ value: credential.table.value, purgedAt: credential.table.purgedAt })
            .from(credential.table)
            .where(eq(credential.table.id, expired.id));
        expect(kept).toEqual({ value: null, purgedAt: later });

        // look again and purge after the next window ends
        const controller = recoverable.controller(server);
        const pending = await execute(handled, "create", { custodian: "user-2", value: "later" });
        await execute(handled, "delete", { id: pending.id, revision: pending.revision });
        const week = 7 * 24 * 60 * 60 * 1000;
        const delay = await controller.reconcile("trash", { signal: AbortSignal.timeout(5000) });
        const change = (before: object, after: object) =>
            ({ operation: "update", before, after }) as unknown as Change;
        expect([
            controller.watches,
            controller.keys!(change({ deletionRequestedAt: null }, { deletionRequestedAt: 1 })),
            controller.keys!(change({ value: "old" }, { value: "new", deletionRequestedAt: null })),
            await controller.list(),
            delay! > week - 60_000 && delay! <= week,
        ]).toEqual([[handled.table], ["trash"], [], ["trash"], true]);
        await database
            .update(credential.table)
            .set({ deletionRequestedAt: Date.now() - week - 1 })
            .where(eq(credential.table.id, pending.id));
        expect(
            await controller.reconcile("trash", { signal: AbortSignal.timeout(5000) }),
        ).toBeUndefined();
        expect(await auditedActions(database, "system")).toEqual([
            "Credential.purge",
            "Credential.purge",
        ]);
    },
);

test("refuse a host window on a type without recoverable deletion", () => {
    const plain = defineObject({
        name: "plain",
        plural: "plains",
        scope: space,
        fields: {},
        permissions: [],
    });

    expect(() => recoverable.within(plain, { days: 1 })).toThrow(
        new TypeError("object plain is not recoverable"),
    );
});
