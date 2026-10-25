import { testSimulation } from "../src/test/index.ts";
import { defineObject, field, ObjectDescription, type ObjectType } from "../src/index.ts";
import { Package } from "@destack/package";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { none, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";
import { definePackage } from "@destack/object";
import { testServerContext } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { Scope, type Uplink } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { ShareableMethod } from "../src/method/shareable.ts";
import { QUERIES_PUBLICATION } from "../src/replica/replica.ts";
import { ObjectStore, implementPackage } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { request } from "./fixture/request.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space whose partition the cards live in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000a1");

/** Cards their owners mark in a space. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({ create: method.create("write", { fields: [] }) }),
);

/** A room inside the space, a separate partition. */
const room = defineObject({
    name: "room",
    plural: "rooms",
    scope: space,
    isScope: true,
    partition: true,
    fields: {},
    permissions: { read: none() },
});

/** The room the scenarios serve a partition of. */
const ROOM = "room-01996ab0-0000-7000-8000-0000000000a3";

/** The package of the space's rooms. */
const roomsPackage = definePackage({ objects: { room } });

/** The database of the space's rooms. */
const roomsDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { room }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

/** An uplink streaming and receiving nothing, named by who it stands for. */
function idle(name: string): Uplink {
    return {
        confirm: async () => {},
        stream: () => {
            throw new TypeError(`${name} streams no replicas`);
        },
        receive: () => Promise.reject(new TypeError(`${name} receives no changes`)),
        call: () => Promise.reject(new TypeError("the fixture answers no queries")),
    };
}

/** The database of the space's cards. */
const cardsDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { card }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

test("serve a call in the store's partition or in any partition, and point one in another partition to the partition serving it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store the space's cards, served by a store per partition it may serve
    const storage = await TestDatabase.create(cardsDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const events = await testEvents(storage.database, world);
    const serve = async (partition?: string) =>
        await TestObjectStore.at(space.reference(Scope.universe.id, spaceId), {
            chain: async () => undefined,
            environment: world,
            principal: spacePrincipal(spaceId),
            servers: serversOf([card]),
            database: storage.database,
            callKey: testCallKey,
            events,
            origin: { package: card.package },
            ...(partition === undefined ? {} : { partitions: [partition] }),
        });
    const create = (store: ObjectStore) =>
        request(
            store,
            card,
            "create",
            spaceId,
            {},
            userContext(users.frank, spaceId, { environment: world }),
        );

    // serve the space's partition, any partition, and refuse another partition's store
    const local = await create(await serve(spaceId));
    const any = await create(await serve());
    expect([
        local.scope,
        any.scope,
        await refusal(create(await serve("space-01996ab0-0000-7000-8000-0000000000a2"))),
    ]).toEqual([
        spaceId,
        spaceId,
        [
            "MISDIRECTED_REQUEST",
            "PARTITION_MISDIRECTED",
            `${spaceId} is served by partition ${spaceId}`,
        ],
    ]);
});

test("follow an installation's root chain from its machine, and a partition's chain below the root from the installation's root through its package, one replica per link", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // create a room partitioning inside the space
    const storage = await TestDatabase.create(roomsDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    await new AccessFixture(storage.database, world).replicateScope(
        room.reference(spaceId, ROOM),
        room.policy,
    );

    // serve the installation at its root and in the room, recording the publishers it calls
    const called: string[] = [];
    const machine = idle("machine");
    const root = idle("root");
    const subscriptions = async (partition: string) => {
        const store = new ObjectStore({
            servers: [implementPackage(roomsPackage, {})],
            chain: async () => ({
                kind: "follower",
                parent: partition === spaceId ? Scope.universe.id : spaceId,
            }),
            environment: world,
            origin: { package: Package.of(roomsPackage) },
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            database: storage.database,
            server: {
                ...(await testServerContext(Package.of(roomsPackage), spaceId, world)),
                partition,
                publisher: machine,
                publisherAt: (address, packageId) => {
                    called.push(`${address} ${packageId}`);

                    return root;
                },
            },
        });
        const listed = present(store.subscriber?.subscriptions, "the store's subscriptions");
        const entries = await listed(Snapshot.live(storage.database));

        return entries.map((entry) => [
            entry.subscription.name,
            entry.subscription.scope,
            entry.publisher === machine ? "machine" : entry.publisher === root ? "root" : "other",
        ]);
    };

    // follow each link's chain, which at the universe has the access grants delegated into it
    expect([await subscriptions(spaceId), await subscriptions(ROOM), called]).toEqual([
        [
            [`chain:${spaceId}`, spaceId, "machine"],
            [`chain:${spaceId}`, "universe", "machine"],
        ],
        [
            [`chain:${ROOM}`, ROOM, "root"],
            [`chain:${ROOM}`, spaceId, "root"],
            [`chain:${ROOM}`, "universe", "root"],
        ],
        [`fixture--${spaceId} ${roomsPackage.id}`],
    ]);
});

test("list the types a home and a follower decide in each chain subscription, adding an installed type to each", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store the space's cards and markers, with markers installed later
    const marker = defineObject({
        name: "marker",
        plural: "markers",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner"), write: relation("owner") },
    });
    const declared = defineDatabase({
        name: "main",
        ...tablesOf({
            packages: [objectPackage, { objects: { card, marker }, tables: [] }],
            follows: [],
            tables: [],
        }),
    });
    const storage = await TestDatabase.create(declared, { environment: world, isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const idles = { machine: idle("machine"), root: idle("root") };
    const chains = async (types: readonly ObjectType[], kind: "home" | "follower") => {
        const store = new ObjectStore({
            servers: serversOf(types),
            chain: async () => ({ kind, parent: Scope.universe.id }),
            environment: world,
            origin: { package: card.package },
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            database: storage.database,
            server: {
                ...(await testServerContext(card.package, spaceId, world)),
                publisher: idles.machine,
                publisherAt: () => idles.root,
            },
        });
        const listed = await store.sink.chains(spaceId, Snapshot.live(storage.database));

        return listed
            .map((entry) => entry.subscription)
            .filter((subscription) => subscription.publication !== QUERIES_PUBLICATION)
            .map((subscription) => [
                subscription.publication,
                subscription.scope,
                subscription.parameters,
            ]);
    };

    // list the chain subscriptions before and after installing markers, for a home and a follower
    const home = [await chains([card], "home"), await chains([card, marker], "home")];
    const follower = [await chains([card], "follower"), await chains([card, marker], "follower")];

    // carry the decided types in every chain subscription, the installed marker beside the card
    const chain = (scope: string, local: readonly ObjectType[]) => [
        "chain",
        scope,
        {
            scopes: [spaceId, Scope.universe.id],
            local: local.map((type) => type.typeReference),
            types: [],
            via: [],
            between: [],
        },
    ];
    expect({ home, follower }).toEqual({
        home: [[chain(Scope.universe.id, [card])], [chain(Scope.universe.id, [card, marker])]],
        follower: [
            [chain(spaceId, [card]), chain(Scope.universe.id, [card])],
            [chain(spaceId, [card, marker]), chain(Scope.universe.id, [card, marker])],
        ],
    });
});

test("derive the roles of a shared partition without sharing methods on its row, and refuse one declaring them", () => {
    // share a room through the roles, whose access lives in the room itself
    const shared = defineObject({
        name: "room",
        plural: "rooms",
        scope: space,
        isScope: true,
        partition: true,
        fields: {},
        permissions: { read: none() },
        shareable: { isPublic: true },
    });

    // derive the roles' relations, and no sharing methods
    expect({
        relations: Object.keys(shared.policy.definition.relations).toSorted(),
        sharing: Object.keys(shared.methods).filter((name) =>
            ["relationships", "grant", "revoke", "invite", "accept", "withdraw"].includes(name),
        ),
    }).toEqual({
        relations: ["commenter", "editor", "owner", "public", "viewer"],
        sharing: [],
    });

    // refuse a room declaring a sharing method on its row
    expect(() =>
        ObjectDescription.of(
            defineObject(
                {
                    name: "room",
                    plural: "rooms",
                    scope: space,
                    isScope: true,
                    partition: true,
                    fields: {},
                    permissions: { read: none(), manage: none() },
                },
                () => ({
                    explain: ShareableMethod.methods({ grant: "manage", read: "read" }).explain,
                }),
            ),
        ),
    ).toThrow(
        new TypeError("object room must share a partitioned scope through its relationships"),
    );
});
