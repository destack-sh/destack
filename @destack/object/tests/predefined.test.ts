import { testSimulation } from "../src/test/index.ts";
import { ObjectReference, type Call, RequestId, Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { AccessContext, accessRole, Caller, none, Role, Manager } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { testCallKey } from "@destack/object/test";
import { defineObject, field, ObjectPolicy, role } from "../src/index.ts";
import { testEvents, serversOf } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The space the tiers live in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The free tier, the same row in every database. */
const FREE = schema.identifier("tier").parse("tier-00000000-0000-7000-8000-000000000001");

/** Plan tiers, the free one predefined. */
const tier = defineObject(
    {
        name: "tier",
        plural: "tiers",
        title: t`${plural(argument("count"), { one: "Tier", other: "Tiers" })}`,
        scope: space,
        fields: { name: field.string() },
        predefined: [
            {
                id: FREE,
                createdAt: 0,
                ...ObjectReference.columns("createdBy", undefined),
                updatedAt: 0,
                ...ObjectReference.columns("updatedBy", undefined),
                revision: 1,
                tags: {},
                ...Manager.values(null),
                detachedAt: null,
                scope: spaceId,
                name: "free",
            },
        ],
        access: {
            permissions: { read: none(), manage: none() },
        },
    },
    (method) => ({
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage", { fields: ["name"] }),
        updateMany: method.updateMany("manage", { fields: ["name"], match: ["name"] }),
        delete: method.delete("manage"),
    }),
);

test("store a type's predefined rows in its table, and refuse every call writing them", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the tiers to their space
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { tier }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([tier]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: tier.package },
    });
    const caller = AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now());
    const mutate = (call: Call) =>
        refusal(store.mutate(caller, { id: RequestId.create(world), calls: [call] }));

    // list the free tier, then write it by identifier, by match and by a chosen identifier
    const { items } = await store.query(caller, tier.call("list", { spaceId }));
    const predefined = `cannot write tier '${FREE}': it is predefined`;
    expect({
        listed: items.map((item) => [item.id, item.name]),
        updated: await mutate(tier.call("update", { spaceId, id: FREE, name: "gratis" })),
        matched: await mutate(
            tier.call("updateMany", { spaceId, where: { name: "free" }, name: "gratis" }),
        ),
        deleted: await mutate(tier.call("delete", { spaceId, id: FREE })),
        created: await mutate(tier.call("create", { spaceId, id: FREE, name: "free" })),
    }).toEqual({
        listed: [[FREE, "free"]],
        updated: ["PRECONDITION_FAILED", "OBJECT_BLOCKED", predefined],
        matched: ["PRECONDITION_FAILED", "OBJECT_BLOCKED", predefined],
        deleted: ["PRECONDITION_FAILED", "OBJECT_BLOCKED", predefined],
        created: ["PRECONDITION_FAILED", "OBJECT_BLOCKED", predefined],
    });
});

test("store the owner role as the role type's predefined row, refusing other rows over its table", () => {
    // read the roles' predefined rows, and declare roles with separate rows
    expect(role.predefined.map((row) => row["id"])).toEqual([Role.OWNER]);
    const mapping = ObjectPolicy.readMapping(accessRole);
    expect(() =>
        defineObject({
            name: "role",
            plural: "roles",
            title: t`${plural(argument("count"), { one: "Role", other: "Roles" })}`,
            mapping,
            predefined: [],
        }),
    ).toThrow(new TypeError("object 'role' over a foreign table cannot have 'predefined'"));
});
