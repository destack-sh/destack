import { TestObjectStore } from "../src/test/index.ts";
import { testSimulation } from "../src/test/index.ts";
import { type Change, eq, defineDatabase } from "@destack/db";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { reconciliation, TestDatabase } from "@destack/db/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { expect, refusal, onTestFinished, test } from "@destack/test";
import { principal, relation, through } from "@destack/access";

import { schema, present } from "@destack/schema";

import { defineObject, field, type CallableName, type ObjectType } from "../src/index.ts";
import { RecoverableController } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext, users } from "./fixture/user.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The space containing the credentials. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** Credentials that retain their record after a purge, reserved to their custodian. */
const credential = defineObject(
    {
        name: "credential",
        plural: "credentials",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            custodian: field.reference(principal.user),
            value: field.string().optional(),
        },
        recoverable: { within: { minutes: 1 }, by: "write", purge: "purge", retain: "record" },
        permissions: {
            read: relation("owner"),
            write: relation("owner"),
            purge: relation("custodian"),
        },
    },
    (method) => ({
        create: method.create("write", { fields: ["custodianId", "value"] }),
    }),
);

/** The database with the credentials, their access and the journal. */
const credentialDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { credential }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

test("purge a retained record through its handler under its permission or as the system, and restore it within its window", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const { clock } = simulation;
    const world = simulation.environment("test");

    const storage = await TestDatabase.create(credentialDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);

    // serve credentials with a purge handler that destroys the value
    const handled = credential.implement({
        methods: {
            purge: async (call, next) => {
                await call.update({ value: null });

                return next();
            },
        },
    });
    const events = await testEvents(database, world);
    const serve = (object: ObjectType) =>
        TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            chain: async () => undefined,
            principal: spacePrincipal(spaceId),
            servers: serversOf([object]),
            database,
            callKey: testCallKey,
            events,
            origin: {
                package: credential.package,
            },
            environment: world,
        });
    let context = userContext(users.frank, spaceId, { environment: world });
    const execute = <Object extends typeof credential, Name extends CallableName<Object>>(
        object: Object,
        name: Name,
        input: NoInfer<RequestInput<Object, Name>>,
    ) => serve(object).then((store) => request(store, object, name, spaceId, input, context));
    // restore within the declared window
    const created = await execute(handled, "create", {
        custodianId: users.grace,
        value: "hunter2",
    });
    await execute(handled, "delete", { id: created.id, revision: created.revision });
    const restored = await execute(handled, "restore", { id: created.id });
    expect(restored.deletedAt).toBeNull();

    // refuse restoring past the declared window
    const lapsed = await execute(handled, "create", { custodianId: users.grace, value: "old" });
    await execute(handled, "delete", { id: lapsed.id, revision: lapsed.revision });
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(credential.table)
                .set({ deletedAt: clock.now() - 120_000 })
                .where(eq(credential.table.id, lapsed.id)),
    );
    expect(await refusal(execute(handled, "restore", { id: lapsed.id }))).toEqual([
        "CONFLICT",
        "RECOVERY_EXPIRED",
        "credential is past its recovery window",
    ]);

    // reserve purging to the custodian, even for the owner deleting and restoring
    await execute(handled, "delete", { id: created.id, revision: restored.revision });
    expect(await refusal(execute(handled, "purge", { id: created.id }))).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission denied: purge",
    ]);

    // refuse a record-retaining purge without a handler destroying the content
    context = userContext(users.grace, spaceId, { environment: world });
    await expect(execute(credential, "purge", { id: created.id })).rejects.toThrow(
        new TypeError(
            "object credential keeps purged records, so its purge must destroy their content",
        ),
    );

    // destroy the value and retain the record, marked purged
    await execute(handled, "purge", { id: created.id });
    const [purged] = await database
        .select()
        .from(credential.table)
        .where(eq(credential.table.id, created.id));
    // record the owner as the deleter and the custodian as the last writer
    expect(purged).toEqual({
        ...created,
        value: null,
        revision: 5,
        updatedAt: clock.now(),
        ...ObjectReference.columns("updatedBy", principal.user.reference("universe", users.grace)),
        deletedAt: clock.now(),
        ...ObjectReference.columns("deletedBy", principal.user.reference("universe", users.frank)),
        purgedAt: clock.now(),
    });

    // refuse purging and restoring a purged record
    expect(await refusal(execute(handled, "purge", { id: created.id }))).toEqual([
        "CONFLICT",
        "PURGED",
        "credential is purged",
    ]);
    context = userContext(users.frank, spaceId, { environment: world });
    expect(await refusal(execute(handled, "restore", { id: created.id }))).toEqual([
        "CONFLICT",
        "PURGED",
        "credential is purged",
    ]);

    // purge the lapsed record as the system, through its handler, once
    const store = await serve(handled);
    const later = clock.now() + 120_000;
    await simulation.jump(later);
    expect([
        await RecoverableController.purge(store, later),
        await RecoverableController.purge(store, later),
    ]).toEqual([1, 0]);
    expect(await auditedActions(database, "component")).toEqual(["credential.purge"]);
    const [record] = await database
        .select({ value: credential.table.value, purgedAt: credential.table.purgedAt })
        .from(credential.table)
        .where(eq(credential.table.id, lapsed.id));
    expect(record).toEqual({ value: null, purgedAt: later });

    // look again and purge after the next window ends
    const controller = new RecoverableController(store);
    const pending = await execute(handled, "create", {
        custodianId: users.grace,
        value: "later",
    });
    await execute(handled, "delete", { id: pending.id, revision: pending.revision });
    const window = 60_000;
    const delay = present(
        await controller.reconcile("trash", reconciliation(world.clock.timeout(5000))),
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
        controller.keys(updated({ deletedAt: null }, { deletedAt: 1 })),
        controller.keys(
            updated({ value: "old", deletedAt: null }, { value: "new", deletedAt: null }),
        ),
        await controller.list(),
        delay > 0 && delay <= window,
    ]).toEqual([[handled.table], ["trash"], [], ["trash"], true]);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(credential.table)
                .set({ deletedAt: clock.now() - window - 1 })
                .where(eq(credential.table.id, pending.id)),
    );
    expect(
        await controller.reconcile("trash", reconciliation(world.clock.timeout(5000))),
    ).toBeUndefined();
    expect(await auditedActions(database, "component")).toEqual([
        "credential.purge",
        "credential.purge",
    ]);
});

/** Drawers holding letters, trashed with their letters on delete. */
const drawer = defineObject(
    {
        name: "drawer",
        plural: "drawers",
        scope: space,
        controlled: true,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", { fields: [] }),
        delete: method.delete("write"),
    }),
);

/** Letters in a drawer that keep their record after a purge, trashed with their drawer. */
const letter = defineObject(
    {
        name: "letter",
        plural: "letters",
        scope: space,
        nested: { in: drawer, delete: "trash", receive: "write" },
        fields: { text: field.string().optional() },
        recoverable: { within: { minutes: 1 }, by: "write", retain: "record" },
        permissions: { read: through("parent", "read"), write: through("parent", "write") },
    },
    (method) => ({
        create: method.create("write", { fields: ["text"] }),
    }),
);

test("discard a trashed letter once purged, refusing it before", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    // serve drawers and letters as frank in a space, a letter's purge destroying its text
    const purging = letter.implement({
        methods: {
            purge: async (call, next) => {
                await call.update({ text: null });

                return next();
            },
        },
    });
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { drawer, letter }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        principal: spacePrincipal(spaceId),
        servers: serversOf([drawer, purging]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: drawer.package },
        environment: world,
    });
    const context = userContext(users.frank, spaceId, { environment: world });
    const made = await request(store, drawer, "create", spaceId, {}, context);
    const written = await request(
        store,
        letter,
        "create",
        spaceId,
        { parentId: made.id, text: "secret" },
        context,
    );
    await request(
        store,
        letter,
        "delete",
        spaceId,
        { id: written.id, revision: written.revision },
        context,
    );

    // discard the trashed letter as its installation, refused until purged, then removed
    const discard = () =>
        store.mutate(store.as(letter), {
            id: RequestId.create(world),
            calls: [letter.call("discard", letter.target({ scope: spaceId, id: written.id }))],
        });
    const kept = await refusal(discard());
    await request(store, letter, "purge", spaceId, { id: written.id }, context);
    await discard();
    const remaining = await database.select({ id: letter.table.id }).from(letter.table);
    expect([kept, remaining]).toEqual([["CONFLICT", "PURGE_REQUIRED", "letter is not purged"], []]);
});
