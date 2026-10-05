import { type Change, eq, defineDatabase } from "@destack/db";
import { Subject } from "@destack/sync";
import { reconciliation, testCallKey } from "@destack/service/test";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, present } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { defineObject, field, type CallableName, type ObjectType } from "../src/index.ts";
import { ObjectServer, RecoverableController } from "../src/server/index.ts";
import { recoverable } from "../src/trait/recoverable.ts";
import { openSpace, space } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext } from "./fixture/user.ts";
import { any } from "./fixture/match.ts";

/** The space containing the credentials. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

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
    methods: (method) => ({
        get: method.get("read"),
        create: method.create("write", { fields: ["custodianId", "value"] }),
    }),
});

/** The database with the credentials, their access and the journal. */
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
                        .where(
                            eq(
                                credential.table.id,
                                present(call.target, "the purged credential").id,
                            ),
                        );

                    return next();
                },
            }),
            { days: 7 },
        );
        const serve = (object: ObjectType) =>
            new ObjectServer({
                objects: { credential: object },
                database,
                callKey: testCallKey,
                origin: {
                    package: credential.package,
                    service: "test",
                },
            });
        let context = userContext("user-1", spaceId);
        const execute = <Object extends typeof credential, Name extends CallableName<Object>>(
            object: Object,
            name: Name,
            input: Record<string, unknown>,
        ) =>
            serve(object).call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            );
        // restore within the host's window what the declared window no longer restores
        const created = await execute(handled, "create", {
            custodianId: "user-2",
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
        context = userContext("user-2", spaceId);
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
        // name the owner as the deleter and the custodian as the last writer
        expect(purged).toEqual({
            ...created,
            value: null,
            revision: 5,
            updatedAt: any(Number),
            updatedBy: Subject.key(principal.user.reference("universe", "user-2")),
            deletionRequestedAt: any(Number),
            deletedBy: Subject.key(principal.user.reference("universe", "user-1")),
            purgedAt: any(Number),
        });

        // refuse purging and restoring a purged record
        await expect(execute(handled, "purge", { id: created.id })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "credential is purged",
        });
        context = userContext("user-1", spaceId);
        await expect(execute(handled, "restore", { id: created.id })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "credential is purged",
        });

        // purge a record past the host's window as the system, through its handler, once
        const expired = await execute(handled, "create", {
            custodianId: "user-2",
            value: "swordfish",
        });
        await execute(handled, "delete", { id: expired.id, revision: expired.revision });
        const server = serve(handled);
        const later = Date.now() + 8 * 24 * 60 * 60 * 1000;
        expect([
            await RecoverableController.purge(server, later),
            await RecoverableController.purge(server, later),
        ]).toEqual([1, 0]);
        expect(await auditedActions(database, "system")).toEqual(["credential.purge"]);
        const [kept] = await database
            .select({ value: credential.table.value, purgedAt: credential.table.purgedAt })
            .from(credential.table)
            .where(eq(credential.table.id, expired.id));
        expect(kept).toEqual({ value: null, purgedAt: later });

        // look again and purge after the next window ends
        const controller = new RecoverableController(server);
        const pending = await execute(handled, "create", { custodianId: "user-2", value: "later" });
        await execute(handled, "delete", { id: pending.id, revision: pending.revision });
        const week = 7 * 24 * 60 * 60 * 1000;
        const delay = present(
            await controller.reconcile("trash", reconciliation(AbortSignal.timeout(5000))),
            "the wait until the next window ends",
        );
        const updated = (
            before: Partial<typeof pending>,
            after: Partial<typeof pending>,
        ): Change<typeof credential.table> => ({
            transaction: null,
            table: credential.table,
            key: { id: pending.id },
            scope: spaceId,
            changedAt: 1,
            operation: "update",
            before: { ...pending, ...before },
            after: { ...pending, ...after },
            sequence: 1,
        });
        if (controller.keys === undefined) {
            throw new TypeError("the trash controller keys no changes");
        }
        expect([
            controller.watches,
            controller.keys(updated({ deletionRequestedAt: null }, { deletionRequestedAt: 1 })),
            controller.keys(
                updated(
                    { value: "old", deletionRequestedAt: null },
                    { value: "new", deletionRequestedAt: null },
                ),
            ),
            await controller.list(),
            delay > week - 60_000 && delay <= week,
        ]).toEqual([[handled.table], ["trash"], [], ["trash"], true]);
        await database
            .update(credential.table)
            .set({ deletionRequestedAt: Date.now() - week - 1 })
            .where(eq(credential.table.id, pending.id));
        expect(
            await controller.reconcile("trash", reconciliation(AbortSignal.timeout(5000))),
        ).toBeUndefined();
        expect(await auditedActions(database, "system")).toEqual([
            "credential.purge",
            "credential.purge",
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
