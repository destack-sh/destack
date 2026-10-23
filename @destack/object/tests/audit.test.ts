import { expect, onTestFinished, single, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { v7 } from "uuid";
import { Package } from "@destack/package";
import { ObjectStore } from "../src/server/index.ts";
import { defineObject, field } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { auditedCalls } from "./fixture/audit.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The space with the lockers. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** A package hosting other packages' objects in its store. */
const HOST = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000f1",
    name: "@example/host",
    version: "2026.10.0",
});

/** Lockers with audited reads that record the disclosed version. */
const locker = defineObject(
    {
        name: "locker",
        plural: "lockers",
        scope: space,
        audited: { reads: true },
        fields: { owner: field.reference(principal.user).caller(), version: field.integer() },
        permissions: { read: relation("owner") },
    },
    (method) => ({
        open: method.query({
            permission: "read",
            output: schema.object({ version: schema.number(), value: schema.string() }),
            audit: { details: schema.object({ version: schema.number() }) },
        }),
        /** Check the locker with an outside system, an audited external effect. */
        check: method.query({
            permission: "read",
            output: schema.object({ isLocked: schema.boolean() }),
            isAudited: true,
        }),
    }),
);

test("record the version a read disclosed in its recorded call, apart from its value", async () => {
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { locker }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, SPACE_ID);
    const [row] = await database
        .insert(locker.table)
        .values({
            id: schema.identifier("locker").parse(`locker-${v7()}`),
            scope: SPACE_ID,
            ownerId: users.owner,
            version: 3,
            createdAt: 1,
            updatedAt: 1,
        })
        .returning();

    // record each audited read as a call event
    const handled = locker.implement({
        methods: {
            open: async (opening) => ({ version: opening.target.version, value: "secret" }),
        },
    });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([handled]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: {
            package: locker.package,
        },
    });
    const context = userContext(users.owner, SPACE_ID);
    const { id } = present(row, "the inserted locker");

    // record the version in the result, never the value
    expect(
        await store.query(context.access(), handled.call("open", { spaceId: SPACE_ID, id })),
    ).toEqual({
        version: 3,
        value: "secret",
    });
    const recorded = await auditedCalls(database);
    expect(
        recorded.map((read) => [
            read.method,
            read.execution.category,
            read.execution.target,
            read.execution.details,
        ]),
    ).toEqual([["locker.open", "access", { type: "locker", id }, { version: 3 }]]);
});

test("record a call under the package of its method when a store of another package serves it", async () => {
    // serve the lockers from a store whose origin is another package
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { locker }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, SPACE_ID);
    const inserted = await database
        .insert(locker.table)
        .values({
            id: schema.identifier("locker").parse(`locker-${v7()}`),
            scope: SPACE_ID,
            ownerId: users.owner,
            version: 3,
            createdAt: 1,
            updatedAt: 1,
        })
        .returning();
    const handled = locker.implement({
        methods: {
            open: async (opening) => ({ version: opening.target.version, value: "secret" }),
            check: async () => ({ isLocked: true }),
        },
    });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([handled]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: HOST },
    });
    const context = userContext(users.owner, SPACE_ID);
    const { id } = single(inserted);

    // record the audited read and the audited external call under the lockers' package
    await store.query(context.access(), handled.call("open", { spaceId: SPACE_ID, id }));
    const checked = await store.query(
        context.access(),
        handled.call("check", { spaceId: SPACE_ID, id }),
    );
    const recorded = await auditedCalls(database);
    expect([
        checked,
        recorded.map((call) => [call.method, call.execution.context.package.id]),
    ]).toEqual([
        { isLocked: true },
        [
            ["locker.open", locker.package.id],
            ["locker.check", locker.package.id],
        ],
    ]);
});
