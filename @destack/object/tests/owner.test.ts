import { testSimulation } from "../src/test/index.ts";
import {
    type CallableName,
    Claim,
    claimTable,
    defineObject,
    field,
    ObjectDescription,
} from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { none, through, union, universe } from "@destack/access";

import { defineDatabase, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { subjectContext, testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { claim } from "./fixture/claim.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space with the assets. */
const homeId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000021");

/** The owner space of the assets, whose principal has its owner role. */
const holderId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000022");

/** A space that is the owner of nothing in the home. */
const rivalId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000023");

/** Assets stored in one space with another as owner, managed by the roles of the owner space. */
const declared = defineObject(
    {
        name: "asset",
        plural: "assets",
        scope: space,
        owner: "holder",
        fields: {
            /** The owner space of the asset. */
            holder: field.reference(space).relate("holder"),
            /** The asset's name, unique within its owner. */
            name: field.string(),
        },
        indexes: { name: { on: ["holderId", "name"], across: () => universe } },
        permissions: { read: through("holder", "read"), manage: none() },
    },
    (method) => ({
        create: method.create("manage"),
        /** Count the owner's assets, checked as on a new asset of the owner in the input. */
        count: method.mutation({
            permission: "manage",
            target: "scope",
            input: schema.object({ holderId: schema.identifier("space") }),
            output: schema.object({ count: schema.int() }),
        }),
    }),
);

/** The assets, counted by owner. */
const asset = declared.implement({
    methods: {
        count: async (call) => ({
            count: (
                await call.read(declared).findMany({ where: { holderId: call.input.holderId } })
            ).length,
        }),
    },
});

/** The database with the assets, their access, the replicated spaces and the journal. */
const assetDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { asset }, tables: [] }],
        follows: [],
        tables: [claimTable, space.table],
    }),
});

test("map the relation of an object with an owner in the universe and record its names under its owner", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(assetDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // claim the asset's name under its owner, keyed within the universe
    const row = { id: "asset-1", scope: homeId, holderId, name: "lamp" };
    const claims = await Claim.list(asset, row, Snapshot.live(storage.database));
    expect({
        owner: asset.mapping.owner,
        relation: asset.mapping.relations["holder"],
        claims: claims.map((entry) => [entry.key, entry.owner]),
    }).toEqual({
        owner: "holder",
        relation: { column: "holderId", scope: Scope.universe.id },
        claims: [[JSON.stringify([Scope.universe.id, holderId, "lamp"]), holderId]],
    });
});

test("map a reference to an object of another scope type across scopes, and one beside the referring object in its scope", () => {
    // declare teams and vaults in spaces, and items in vaults with a team owner, tagged beside them
    const team = defineObject({
        name: "team",
        plural: "teams",
        scope: space,
        isScope: true,
        fields: {},
        permissions: { read: none() },
    });
    const vault = defineObject({
        name: "vault",
        plural: "vaults",
        scope: space,
        isScope: true,
        fields: {},
        permissions: { read: none() },
    });
    const tag = defineObject({
        name: "tag",
        plural: "tags",
        scope: vault,
        fields: {},
        permissions: { read: none() },
    });
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
    });

    // follow the team in any scope, and the tag in the item's scope
    expect(item.mapping.relations).toEqual({
        team: { column: "teamId", scope: "*" },
        tag: { column: "tagId" },
    });
});

test("refuse an owner that no reference relates", () => {
    expect(() =>
        ObjectDescription.of(
            defineObject({
                name: "orphan",
                plural: "orphans",
                scope: space,
                owner: "holder",
                fields: { name: field.string() },
            }),
        ),
    ).toThrow("object orphan takes its owner from holder, but no reference relates it");
});

test("let the roles of an owner create objects of that owner and run scoped methods for that owner alone", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store assets in a public home with a private holder as owner, beside a rival space
    const storage = await TestDatabase.create(assetDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    for (const id of [homeId, holderId, rivalId]) {
        await openSpace(storage.database, world, id, id === holderId ? "private" : "public");
        await storage.database.transaction(
            async (transaction) =>
                await transaction
                    .insert(space.table)
                    .values({ id, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 }),
        );
    }
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, homeId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(homeId),
        servers: serversOf([asset]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: asset.package },
        claims: { object: claim },
    });
    const call = <Name extends CallableName<typeof asset>>(
        by: string,
        name: Name,
        input: NoInfer<RequestInput<typeof asset, Name>>,
    ) =>
        refusal(
            request(
                store,
                asset,
                name,
                homeId,
                { requestId: RequestId.create(world), ...input },
                subjectContext(spacePrincipal(by), homeId, { environment: world }),
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
        ["NOT_FOUND", "NOT_FOUND", "asset not found"],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: manage"],
    ]);
});

/** Stickers stored in a space without an owner, decided only by the space's roles. */
const sticker = defineObject(
    {
        name: "sticker",
        plural: "stickers",
        scope: space,
        fields: {
            /** The sticker's name. */
            name: field.string(),
        },
        permissions: { read: none(), manage: none() },
    },
    (method) => ({ create: method.create("manage") }),
);

/** The database with the assets beside the stickers. */
const stickerDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { asset, sticker }, tables: [] }],
        follows: [],
        tables: [claimTable, space.table],
    }),
});

test("allow an owner its objects in a private home it cannot see, and refuse the home's other objects", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store assets and stickers in a private home, with the holder as owner, beside a rival space
    const storage = await TestDatabase.create(stickerDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    for (const id of [homeId, holderId, rivalId]) {
        await openSpace(storage.database, world, id, "private");
        await storage.database.transaction(
            async (transaction) =>
                await transaction
                    .insert(space.table)
                    .values({ id, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 }),
        );
    }
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, homeId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(homeId),
        servers: serversOf([asset, sticker]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: asset.package },
        claims: { object: claim },
    });
    const create = <Type extends typeof asset | typeof sticker, Name extends CallableName<Type>>(
        by: string,
        type: Type,
        name: Name,
        input: NoInfer<RequestInput<Type, Name>>,
    ) =>
        refusal(
            request(
                store,
                type,
                name,
                homeId,
                { requestId: RequestId.create(world), ...input },
                subjectContext(spacePrincipal(by), homeId, { environment: world }),
            ),
        );

    // create the home's first asset as its holder, and hide the home from the rival's asset and sticker
    expect([
        await create(holderId, asset, "create", { holderId, name: "lamp" }),
        await create(rivalId, asset, "create", { holderId, name: "desk" }),
        await create(rivalId, sticker, "create", { name: "star" }),
    ]).toEqual([
        "done",
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${homeId} not found`],
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${homeId} not found`],
    ]);
});
