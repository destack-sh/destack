import { testSimulation } from "../src/test/index.ts";
import { users, userName } from "./fixture/user.ts";
import { expect, expectTypeOf, onTestFinished, single, test } from "@destack/test";
import {
    AccessContext,
    accessGrant,
    accessRelationship,
    accessRole,
    Authorization,
    Caller,
    none,
    Policy,
    principal,
    Relationship,
} from "@destack/access";
import { defineDatabase, eq, Transaction } from "@destack/db";
import { AccessFixture } from "@destack/access/test";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { subjectContext, testCallKey } from "@destack/object/test";
import { testEvents, serversOf, TestObjectStore } from "../src/test/index.ts";
import { RequestId, ObjectReference, type Subject, Scope } from "@destack/sync";
import { defineObject, ObjectPolicy, relationship, role } from "../src/index.ts";
import { space, spacePrincipal } from "./fixture/space.ts";
import { CallSite, tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { installation } from "../src/installation/index.ts";

/** The package the space installs beside its base installations. */
const DEPLOY_PACKAGE_ID = schema
    .identifier("package")
    .parse("package-01996ab0-0000-7000-8000-0000000000d1");

/** The space whose roles and members the test follows. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

test("follow a space's roles and the members bound to them as the owner defines and binds them", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { role }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;

    // reference the owner and the space of the owner
    const owner = principal.user.reference("universe", users.owner);
    const object = space.reference("universe", SPACE_ID);

    // serve the space's roles and relationships to the owner
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, SPACE_ID), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([role, relationship]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: {
            package: role.package,
        },
    });
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(space.table)
                .values({ id: SPACE_ID, scope: "universe", createdAt: 1, updatedAt: 1 }),
    );

    // bear the space at its home, its owner bound in the universe and replicated here
    const born = await Transaction.open(database);
    await new Authorization(
        store.authorizer,
        born,
        () => AccessContext.of(Caller.of(owner), world.clock.now()),
        world,
    ).receive(object, []);
    await born.commit();
    await new AccessFixture(database, world).replicateOwner(object, owner);
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const context = subjectContext(principal.user.reference("universe", users.owner), SPACE_ID, {
        environment: world,
        signal: controller.signal,
    });

    // follow the space's roles and the relationships inside it, its owner bound beside its row
    const pages = store.source.relayed(
        {
            ...store.source.queriesPublication.subscription({
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
            revision: { log: "follower", epoch: "follower", sequence: 1 },
        },
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

    // upsert a new role at once and its binding to a member
    const reviewer = await store
        .mutate(context.access(), {
            id: RequestId.create(world),
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
    ).toThrow(
        new TypeError("object role over a foreign table has no parent relation to nest through"),
    );
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
    // run in a world of its own
    const world = testSimulation().environment("test");

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
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, SPACE_ID), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([role, relationship]),
        policies: [space, role, role],
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: role.package },
    });

    // map the role's table once
    const { packageId, name } = role.policy.definition;
    expect(store.authorizer.requireMapping({ packageId, type: name }).table).toBe(role.table);
});

test("delete the grants of an installation as the installation is deleted", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve a space's installations to its owner
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const owner = principal.user.reference("universe", users.owner);

    // bear the space at its home with its base installations, its owner bound in the universe and replicated here
    const store = await TestObjectStore.receive(principal.space.reference("universe", SPACE_ID), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([installation]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: installation.package },
    });
    await new AccessFixture(database, world).replicateOwner(
        principal.space.reference("universe", SPACE_ID),
        owner,
    );

    // install a package from the store's code as its owner, and grant the owner's authority to it beside another delegate
    const caller = subjectContext(owner, SPACE_ID, { environment: world }).access();
    const [created] = await store.mutate(
        caller,
        {
            id: RequestId.create(world),
            calls: [
                installation.call("install", {
                    ...installation.in(SPACE_ID),
                    origin: "installed",
                    packageId: DEPLOY_PACKAGE_ID,
                    name: "deploy",
                }),
            ],
        },
        { site: CallSite.store },
    );
    const delegate = installation.reference(
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
        expiresAt: world.clock.now() + 60_000,
        createdAt: 1,
    });
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(accessGrant)
                .values([
                    grant("grant-01996ab0-0000-7000-8000-000000000001", delegate),
                    grant("grant-01996ab0-0000-7000-8000-000000000002", other),
                ]),
    );

    // delete the installation from the store's code
    await store.mutate(
        caller,
        {
            id: RequestId.create(world),
            calls: [installation.call("delete", { ...installation.in(SPACE_ID), id: delegate.id })],
        },
        { site: CallSite.store },
    );

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

test("make each object a partition when the principal it stands for declares one, deriving no sharing methods", () => {
    // represent a shared scope policy whose objects are partitions, and one whose objects are not
    const site = new Policy(space.package, {
        name: "site",
        permissions: {},
        scope: true,
        partition: true,
    });
    const room = new Policy(space.package, { name: "room", permissions: {}, scope: true });
    const partitioned = defineObject(
        {
            name: "site",
            plural: "sites",
            scope: space,
            represents: site,
            fields: {},
            permissions: { read: none(), manage: none() },
            shareable: {},
        },
        (method) => ({ create: method.create("manage") }),
    );
    const shared = defineObject(
        {
            name: "room",
            plural: "rooms",
            scope: space,
            represents: room,
            fields: {},
            permissions: { read: none(), manage: none() },
            shareable: {},
        },
        (method) => ({ create: method.create("manage") }),
    );

    // inherit the partition on the policy and the type, which then shares through its relationships alone, in its type as well
    expectTypeOf(partitioned.methods).not.toHaveProperty("grant");
    expectTypeOf(shared.methods).toHaveProperty("grant");
    expect({
        partitioned: [
            partitioned.policy.definition.partition,
            partitioned.isPartition,
            Object.keys(partitioned.methods).toSorted(),
        ],
        shared: [shared.policy.definition.partition, shared.isPartition, "grant" in shared.methods],
    }).toEqual({
        partitioned: [true, true, ["create", "delete", "get", "list", "update"]],
        shared: [undefined, false, true],
    });
});
