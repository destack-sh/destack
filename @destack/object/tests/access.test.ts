import { users, userName } from "./fixture/user.ts";
import { expect, onTestFinished, single, test } from "@destack/test";
import {
    AccessContext,
    accessGrant,
    accessRelationship,
    accessRole,
    Authorization,
    Caller,
    Policy,
    principal,
    Relationship,
} from "@destack/access";
import { defineDatabase, eq, Transaction } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { subjectContext, testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { RequestId, ObjectReference, type Subject } from "@destack/sync";
import { ObjectStore } from "../src/server/index.ts";
import { defineObject, field, ObjectPolicy, relationship, role } from "../src/index.ts";
import { space, spacePrincipal } from "./fixture/space.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The space whose roles and members the test follows. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

test("follow a space's roles and members as the owner defines and binds them", async () => {
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { role }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;

    // reference the owner and the space of the owner
    const owner = principal.user.reference("universe", users.owner);
    const object = space.reference("universe", SPACE_ID);

    // serve the space's roles and relationships to the owner
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([role, relationship]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: {
            package: role.package,
        },
    });
    await database
        .insert(space.table)
        .values({ id: SPACE_ID, scope: "universe", createdAt: 1, updatedAt: 1 });
    const creating = await Transaction.open(database);
    await new Authorization(store.authorizer, creating, () =>
        AccessContext.of(Caller.of(owner), Date.now()),
    ).create(object, { owner });
    await creating.commit();
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const context = subjectContext(principal.user.reference("universe", users.owner), SPACE_ID, {
        signal: controller.signal,
    });

    // take the owner role and its binding from the snapshot
    const pages = store.source.relayed(
        store.source.queriesPublication.subscription({
            name: "objects",
            scope: SPACE_ID,
            target: SPACE_ID,
            parameters: {
                queries: {
                    roles: { object: "role" },
                    members: { object: "relationship" },
                },
            },
        }),
        context,
    );
    const next = async () => {
        // skip the pages without changes
        let read = await pages.next();
        while (read.done !== true && read.value.changes.length === 0) {
            read = await pages.next();
        }
        if (read.done === true) {
            throw new TypeError("the sync stream ended before a change");
        }
        const page = read.value;

        // label each role by its name, and each relationship by its subject
        return page.changes.map((change) => [
            change.table,
            change.operation,
            change.row["name"] ?? userName(schema.string().parse(change.row["subjectId"])),
        ]);
    };
    expect(await next()).toEqual([["destack__access__relationship", "insert", "owner"]]);

    // upsert a new role at once and its binding to a member
    const reviewer = await store
        .mutate(context.access(), {
            id: RequestId.create(Date.now()),
            calls: [
                role.call("create", {
                    scope: SPACE_ID,
                    name: "reviewer",
                    description: "Reviews the space",
                    permissions: [],
                }),
            ],
        })
        .then(([written]) => written);
    expect(await next()).toEqual([["destack__access__role", "insert", "reviewer"]]);
    const granting = await Transaction.open(database);
    const authorization = await store.authorize(granting, SPACE_ID, context.access());
    const before = authorization.isCurrent;
    await Relationship.grant(
        {
            object,
            role: reviewer.id,
            subject: principal.user.reference("universe", users.member),
        },
        authorization,
    );
    const after = authorization.isCurrent;
    await granting.commit();

    // see the binding, and the authorization outdated by the write it made
    expect([await next(), before, after]).toEqual([
        [["destack__access__relationship", "insert", "member"]],
        true,
        false,
    ]);

    // read the role back with its permissions, and the binding as granted
    const bound = single(
        await database
            .select()
            .from(accessRelationship)
            .where(eq(accessRelationship.roleId, reviewer.id)),
    );
    expect([
        await store.query(context.access(), role.call("get", { scope: SPACE_ID, id: reviewer.id })),
        await store.query(
            context.access(),
            relationship.call("get", { scope: SPACE_ID, id: bound.id }),
        ),
    ]).toEqual([
        {
            id: reviewer.id,
            scope: SPACE_ID,
            name: "reviewer",
            description: "Reviews the space",
            permissions: [],
            revision: reviewer.revision,
        },
        Relationship.decode(bound),
    ]);
});

test("refuse traits on an object type over a foreign table", () => {
    // declare roles that would be recoverable
    expect(() =>
        defineObject({
            name: "role",
            plural: "roles",
            mapping: ObjectPolicy.readMapping(accessRole),
            permissions: ["delete"],
            recoverable: { by: "delete", within: { days: 30 } },
        }),
    ).toThrow(new TypeError("object role over a foreign table cannot have recoverable"));
});

test("nest objects over a foreign table only through its mapping's parent relation", () => {
    // declare roles nested under any object, whose access mapping relates no parent
    expect(() =>
        defineObject({
            name: "role",
            plural: "roles",
            mapping: ObjectPolicy.readMapping(accessRole),
            nested: { in: "any", receive: "role" },
        }),
    ).toThrow(new TypeError("object role over a foreign table nests through no parent relation"));
});

test("live in the universe when listed there or when living in any scope, and not inside one scope type", () => {
    // read where a universe type, an any-scope type and a space type live
    const universal = defineObject({ name: "anchor", plural: "anchors", scope: "universe" });
    const anywhere = defineObject({ name: "badge", plural: "badges" });
    const spaced = defineObject({ name: "marker", plural: "markers", scope: space });

    // accept the first two in the universe, the any-scope one living in any scope
    expect(
        [universal, anywhere, spaced].map((type) => [type.isInUniverse, type.isAnyScope]),
    ).toEqual([
        [true, false],
        [true, true],
        [false, false],
    ]);
});

test("authorize an object type that the objects and the policies list several times", async () => {
    // serve roles and list them twice among the policies, as composed extensions do
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { role }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([role, relationship]),
        policies: [space, role, role],
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: role.package },
    });

    // map the role's table once
    const { packageId, name } = role.policy.definition;
    expect(store.authorizer.requireMapping({ packageId, type: name }).table).toBe(role.table);
});

/** A space's installations, each an installation principal. */
const installed = defineObject(
    {
        name: "installation",
        plural: "installations",
        scope: space,
        represents: principal.installation,
        fields: { name: field.string() },
        permissions: ["read", "write"],
    },
    (method) => ({
        create: method.create("write"),
        delete: method.delete("write"),
    }),
);

test("delete the grants of an installation as the installation is deleted", async () => {
    // serve a space's installations to its owner
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { installed, role }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([installed]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: installed.package },
    });
    await database
        .insert(space.table)
        .values({ id: SPACE_ID, scope: "universe", createdAt: 1, updatedAt: 1 });
    const owner = principal.user.reference("universe", users.owner);
    const creating = await Transaction.open(database);
    await new Authorization(store.authorizer, creating, () =>
        AccessContext.of(Caller.of(owner), Date.now()),
    ).create(space.reference("universe", SPACE_ID), { owner });
    await creating.commit();

    // create an installation, and a grant of the owner to it beside another delegate's
    const caller = subjectContext(owner, SPACE_ID).access();
    const [created] = await store.mutate(caller, {
        id: RequestId.create(Date.now()),
        calls: [installed.call("create", { spaceId: SPACE_ID, name: "deploy" })],
    });
    const delegate = installed.reference(
        SPACE_ID,
        schema.looseObject({ id: schema.string() }).parse(created).id,
    );
    const other = principal.user.reference("universe", users.other);
    const grant = (id: string, to: Subject) => ({
        id: schema.identifier("grant").parse(id),
        scope: "universe",
        ...ObjectReference.columns("delegator", owner),
        ...ObjectReference.columns("delegate", to),
        permissions: [],
        expiresAt: Date.now() + 60_000,
        createdAt: 1,
    });
    await database
        .insert(accessGrant)
        .values([
            grant("grant-01996ab0-0000-7000-8000-000000000001", delegate),
            grant("grant-01996ab0-0000-7000-8000-000000000002", other),
        ]);

    // delete the installation
    await store.mutate(caller, {
        id: RequestId.create(Date.now()),
        calls: [installed.call("delete", { spaceId: SPACE_ID, id: delegate.id })],
    });

    // leave only the other delegate's grant
    const remaining = await database.select().from(accessGrant);
    expect(remaining.map((row) => ObjectReference.at("delegate", row))).toEqual([other]);
});

test("leave a represented policy's relations whole, a concealed one concealed", () => {
    // represent a policy whose holders stay concealed
    const badge = new Policy(space.package, {
        name: "badge",
        relations: { holder: { subjects: [principal.user], concealed: true, onDelete: "cascade" } },
        permissions: {},
    });
    const represented = defineObject({
        name: "badge",
        plural: "badges",
        scope: space,
        represents: badge,
        fields: {},
        permissions: [],
    });

    expect(represented.policy.definition.relations["holder"]).toEqual(
        badge.definition.relations["holder"],
    );
});
