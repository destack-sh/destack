import { expect, onTestFinished, refusal, test } from "@destack/test";
import { none, through, union, universe } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { defineDatabase, Snapshot } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Clock, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { subjectContext, testCallKey } from "@destack/service/test";
import { type CallableName, Claim, claimTable, defineObject, field } from "../src/index.ts";
import { Scope } from "@destack/sync";
import { DatabaseClaimStore, ObjectStore } from "../src/server/index.ts";
import { outbox } from "@destack/service/outbox";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { request } from "./fixture/request.ts";

/** The space keeping the assets. */
const homeId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000021");

/** The space owning the assets, whose own principal holds its owner role. */
const holderId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000022");

/** A space owning nothing kept in the home. */
const rivalId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000023");

/** Assets kept in one space and owned by another, managed by the roles of the owning space. */
const declared = defineObject({
    name: "asset",
    plural: "assets",
    scope: space,
    owner: "holder",
    fields: {
        /** The space owning the asset. */
        holder: field.reference(space).relate("holder"),
        /** The asset's name, unique within its owner. */
        name: field.string(),
    },
    indexes: { name: { on: ["holderId", "name"], unique: true, across: () => universe } },
    permissions: { read: through("holder", "read"), manage: none() },
}).methods((method) => ({
    create: method.create("manage"),
    /** Count the owner's assets, checked as on a new asset of the owner the input names. */
    count: method.mutation({
        permission: "manage",
        target: "none",
        input: schema.object({ holderId: schema.identifier("space") }),
        output: schema.object({ count: schema.int() }),
    }),
}));

/** The assets, counted by owner. */
const asset = declared.implement({
    methods: {
        count: async (call) => ({
            count: (await call.read(declared).rows({ holderId: call.input.holderId })).length,
        }),
    },
});

/** The database with the assets, their access, the copied spaces and the journal. */
const assetDatabase = defineDatabase({
    name: "main",
    tables: [journal, outbox, claimTable, space.table, ...asset.tables],
});

test("map an owned object's relation in the universe and record its names under its owner", async () => {
    const storage = await TestDatabase.create("sqlite", assetDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());

    // claim the asset's name under its owner, keyed within the universe
    const row = { id: "asset-1", scope: homeId, holderId, name: "lamp" };
    const claims = await Claim.list(asset, row, Snapshot.live(storage.database));
    expect({
        owner: asset.mapping.owner,
        relation: asset.mapping.relations["holder"],
        claims: claims.map((claim) => [claim.key, claim.scope]),
    }).toEqual({
        owner: "holder",
        relation: { column: "holderId", scope: Scope.universe.id },
        claims: [[JSON.stringify([Scope.universe.id, holderId, "lamp"]), holderId]],
    });
});

test("map a reference to an object of another scope type across scopes, and one beside the referring object in its scope", () => {
    // declare teams and vaults inside spaces, and items in vaults owned by a team and tagged beside them
    const team = defineObject({
        name: "team",
        plural: "teams",
        scope: space,
        isScope: true,
        fields: {},
        permissions: { read: none() },
    }).methods();
    const vault = defineObject({
        name: "vault",
        plural: "vaults",
        scope: space,
        isScope: true,
        fields: {},
        permissions: { read: none() },
    }).methods();
    const tag = defineObject({
        name: "tag",
        plural: "tags",
        scope: vault,
        fields: {},
        permissions: { read: none() },
    }).methods();
    const item = defineObject({
        name: "item",
        plural: "items",
        scope: vault,
        owner: "team",
        fields: {
            team: field.reference(team).relate("team"),
            tag: field.reference(tag).relate("tag"),
        },
        permissions: { read: union(through("team", "read"), through("tag", "read")) },
    }).methods();

    // follow the team in any scope, and the tag in the item's own scope
    expect(item.mapping.relations).toEqual({
        team: { column: "teamId", scope: "*" },
        tag: { column: "tagId" },
    });
});

test("refuse an owner that no reference relates", () => {
    expect(() =>
        defineObject({
            name: "orphan",
            plural: "orphans",
            scope: space,
            owner: "holder",
            fields: { name: field.string() },
        }).methods(),
    ).toThrow("object orphan is owned through holder, which no reference relates");
});

test.each(TEST_DIALECTS)(
    "let the roles of an owner create owned objects and run scoped methods for that owner alone, on %s",
    async (dialect) => {
        // keep assets in a public home, owned by a private holder, beside a rival space, with each space's copied row
        const storage = await TestDatabase.create(dialect, assetDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        for (const id of [homeId, holderId, rivalId]) {
            await openSpace(storage.database, id, id === holderId ? "private" : "public");
            await storage.database
                .insert(space.table)
                .values({ id, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 });
        }
        const store = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(homeId),
            objects: { asset },
            database: storage.database,
            callKey: testCallKey,
            origin: { package: asset.package, service: "test" },
            claims: new DatabaseClaimStore(storage.database, Clock.system),
        });
        const call = <Name extends CallableName<typeof asset>>(
            by: string,
            name: Name,
            input: object,
        ) =>
            refusal(
                request(
                    store,
                    asset,
                    name,
                    { spaceId: homeId, requestId: RequestId.create(Date.now()), ...input },
                    subjectContext(spacePrincipal(by), homeId),
                ),
            );

        // create and count as the holder, hide its assets from the rival, and refuse the rival's count openly
        expect([
            await call(holderId, "create", { holderId, name: "lamp" }),
            await call(holderId, "count", { holderId }),
            await call(rivalId, "create", { holderId, name: "desk" }),
            await call(rivalId, "count", { holderId }),
        ]).toEqual([
            "done",
            "done",
            ["NOT_FOUND", "asset not found"],
            ["FORBIDDEN", "permission denied: manage"],
        ]);
    },
);
