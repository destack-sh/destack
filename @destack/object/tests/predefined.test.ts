import { ObjectReference } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Caller, none, Role } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { Clock, schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { defineObject, field, Intrinsic, type ObjectCall } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";

/** The space the tiers live in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The free tier, the same row in every database. */
const FREE = schema.identifier("tier").parse("tier-00000000-0000-7000-8000-000000000001");

/** Plan tiers, the free one predefined. */
const tier = defineObject({
    name: "tier",
    plural: "tiers",
    scope: space,
    fields: { name: field.string() },
    permissions: { read: none(), manage: none() },
    predefined: [
        {
            id: FREE,
            createdAt: 0,
            ...ObjectReference.columns("createdBy", undefined),
            updatedAt: 0,
            ...ObjectReference.columns("updatedBy", undefined),
            revision: 1,
            tags: {},
            scope: spaceId,
            name: "free",
        },
    ],
}).methods((method) => ({
    list: method.list("read"),
    create: method.create("manage"),
    update: method.update("manage", { fields: ["name"] }),
    updateMany: method.updateMany("manage", { fields: ["name"], match: ["name"] }),
    delete: method.delete("manage"),
}));

test("keep a type's predefined rows in its table, and refuse every call writing them", async () => {
    // serve the tiers to their space
    const storage = await TestDatabase.create("sqlite", [...callTables, ...tier.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([tier]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: tier.package },
    });
    const caller = Caller.of(spacePrincipal(spaceId), Date.now());
    const mutate = (call: ObjectCall) =>
        refusal(store.mutate(caller, { id: RequestId.create(Date.now()), calls: [call] }));

    // list the free tier, then write it by identifier, by match and by a chosen identifier
    const { items } = await store.query(caller, tier.call("list", { spaceId }));
    const predefined = `tier ${FREE} is predefined, so no call writes it`;
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
        updated: ["CONFLICT", predefined],
        matched: ["CONFLICT", predefined],
        deleted: ["CONFLICT", predefined],
        created: ["CONFLICT", predefined],
    });
});

test("keep the owner role as the role type's predefined row, refusing other rows over its table", () => {
    // read the roles' predefined rows, and declare roles with rows of their own
    const role = defineObject(Intrinsic.role.definition(space)).methods(Intrinsic.role.methods);
    expect(role.predefined.map((row) => row["id"])).toEqual([Role.OWNER]);
    expect(() =>
        defineObject({ ...Intrinsic.role.definition(space), predefined: [] }).methods(),
    ).toThrow(new TypeError("object role over a foreign table cannot have predefined"));
});
