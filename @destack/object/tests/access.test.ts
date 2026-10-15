import { users, userName } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import {
    AccessContext,
    accessGrant,
    Authorization,
    Caller,
    Policy,
    principal,
} from "@destack/access";
import { defineDatabase, Transaction } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";

import { subjectContext, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { RequestId } from "@destack/runtime/request";
import { ObjectStore } from "../src/server/index.ts";
import { defineObject, field, Intrinsic } from "../src/index.ts";
import { space, spacePrincipal } from "./fixture/space.ts";
import { ObjectReference, type Subject } from "@destack/sync";

/** The space whose roles and members the test follows. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** A space's roles. */
const role = defineObject(Intrinsic.role.definition(space)).methods(Intrinsic.role.methods);
/** A space's relationships. */
const relationship = defineObject(Intrinsic.relationship.definition(space)).methods();

test.for(TEST_DIALECTS)(
    "follow a space's roles and members as the owner defines and binds them on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "main",
                tables: [...role.tables, space.table, ...callTables],
            }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        const database = storage.database;

        // reference the owner and the space it owns
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
        const context = subjectContext(
            principal.user.reference("universe", users.owner),
            SPACE_ID,
            {
                signal: controller.signal,
            },
        );

        // keep the owner role and its binding from the snapshot
        const pages = store.source.relayed(
            store.source.queriesPublication.subscription({
                name: "objects",
                scope: SPACE_ID,
                below: SPACE_ID,
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

            // name each role by its name, and each relationship by its subject
            return page.changes.map((change) => [
                change.table,
                change.operation,
                change.row["name"] ?? userName(schema.string().parse(change.row["subjectId"])),
            ]);
        };
        expect(await next()).toEqual([["destack__access__relationship", "insert", "owner"]]);

        // keep a new role at once and its binding to a member
        const reviewer = await store
            .mutate(context.access(), {
                id: RequestId.create(Date.now()),
                calls: [
                    role.call("create", {
                        spaceId: SPACE_ID,
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
        await authorization.grant({
            object,
            role: reviewer.id,
            subject: principal.user.reference("universe", users.member),
        });
        await granting.commit();
        expect(await next()).toEqual([["destack__access__relationship", "insert", "member"]]);
    },
);

test("refuse traits on an object type over a foreign table", () => {
    // declare roles that would be recoverable
    const recoverable = defineObject({
        ...Intrinsic.role.definition(space),
        recoverable: { by: "delete", within: { days: 30 } },
    });

    expect(() => recoverable.methods(Intrinsic.role.methods)).toThrow(
        new TypeError("object role over a foreign table cannot have recoverable"),
    );
});

test("authorize an object type that the objects and the policies list several times", async () => {
    // serve roles and list them twice among the policies, as composed extensions do
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "main", tables: [...role.tables, space.table, ...callTables] }),
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

/** A space's tokens, each a personal access token principal. */
const token = defineObject({
    name: "personal-access-token",
    identity: "token",
    plural: "tokens",
    scope: space,
    represents: principal.personalAccessToken,
    fields: { name: field.string() },
    permissions: ["read", "write"],
}).methods((method) => ({
    create: method.create("write"),
    delete: method.delete("write"),
}));

test("delete the grants of a token as the token is deleted", async () => {
    // serve a space's tokens to its owner
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...token.tables, ...role.tables, space.table, ...callTables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([token]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: token.package },
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

    // create a token, and a grant of the owner to it beside another delegate's
    const caller = subjectContext(owner, SPACE_ID).access();
    const [created] = await store.mutate(caller, {
        id: RequestId.create(Date.now()),
        calls: [token.call("create", { spaceId: SPACE_ID, name: "deploy" })],
    });
    const delegate = token.reference(
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

    // delete the token
    await store.mutate(caller, {
        id: RequestId.create(Date.now()),
        calls: [token.call("delete", { spaceId: SPACE_ID, id: delegate.id })],
    });

    // keep only the other delegate's grant
    const kept = await database.select().from(accessGrant);
    expect(kept.map((row) => ObjectReference.at("delegate", row))).toEqual([other]);
});

test("keep a represented policy's relations whole, a concealed one concealed", () => {
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
    }).methods();

    expect(represented.policy.definition.relations["holder"]).toEqual(
        badge.definition.relations["holder"],
    );
});
