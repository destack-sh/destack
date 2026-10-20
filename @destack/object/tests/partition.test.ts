import { defineObject, field, ObjectDescription, type ObjectType } from "../src/index.ts";
import { Package } from "@destack/package";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { none, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase, Snapshot } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { nth, present, schema } from "@destack/schema";
import { definePackage } from "@destack/runtime";
import { testServerContext } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { Scope, type Uplink } from "@destack/sync";
import { testCallKey } from "@destack/runtime/test";
import { ShareableMethod } from "../src/method/shareable.ts";
import { QUERIES_PUBLICATION } from "../src/replica/replica.ts";
import { ObjectStore, defineServer } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { request } from "./fixture/request.ts";
import { userContext, users } from "./fixture/user.ts";
import { Clock } from "@destack/environment";

/** The space whose partition the cards live in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000a1");

/** Cards their owners pin in a space. */
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

/** A room inside the space, a partition of its own. */
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
const roomsDatabase = defineDatabase({ name: "main", tables: [...room.tables, ...callTables] });

/** Seats in a room, served by a package replicating the rooms. */
const seat = defineObject({
    name: "seat",
    plural: "seats",
    scope: room,
    fields: {},
    permissions: { read: none() },
});

/** The package of the rooms' seats. */
const seatsPackage = definePackage({ objects: { seat } });

/** The database of the rooms' seats, replicating the rooms. */
const seatsDatabase = defineDatabase({
    name: "main",
    tables: [...seat.tables, ...callTables],
    replicated: [room.table],
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
const cardsDatabase = defineDatabase({ name: "main", tables: [...card.tables, ...callTables] });

test.each(TEST_DIALECTS)(
    "serve a call in the store's own partition or in any partition, and point one in another partition to the partition serving it on %s",
    async (dialect) => {
        // store the space's cards, served by a store per partition it may serve
        const storage = await TestDatabase.create(dialect, cardsDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const events = await testEvents(storage.database);
        const serve = (partition?: string) =>
            new ObjectStore({
                chain: async () => undefined,
                clock: Clock.system,
                principal: spacePrincipal(spaceId),
                servers: serversOf([card]),
                database: storage.database,
                callKey: testCallKey,
                events,
                origin: { package: card.package },
                ...(partition === undefined ? {} : { partition }),
            });
        const create = (store: ObjectStore) =>
            request(store, card, "create", spaceId, {}, userContext(users.frank, spaceId));

        // serve the space's partition, any partition, and refuse another partition's store
        const own = await create(serve(spaceId));
        const any = await create(serve());
        expect([
            own.scope,
            any.scope,
            await refusal(create(serve("space-01996ab0-0000-7000-8000-0000000000a2"))),
        ]).toEqual([
            spaceId,
            spaceId,
            ["MISDIRECTED_REQUEST", `${spaceId} is served by partition ${spaceId}`],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "follow an installation's root chain from its machine, and a partition's chain below the root from the installation's root through its own package, one replica per link, on %s",
    async (dialect) => {
        // create a room partitioning inside the space
        const storage = await TestDatabase.create(dialect, roomsDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        await new AccessFixture(storage.database).replicateScope(
            room.reference(spaceId, ROOM),
            "own",
        );

        // serve the installation at its root and in the room, recording the publishers it reaches
        const reached: string[] = [];
        const machine = idle("machine");
        const root = idle("root");
        const subscriptions = async (partition: string) => {
            const store = new ObjectStore({
                servers: [defineServer(roomsPackage, {})],
                chain: async () => ({
                    kind: "follower",
                    parent: partition === spaceId ? Scope.universe.id : spaceId,
                }),
                clock: Clock.system,
                origin: { package: Package.of(roomsPackage) },
                callKey: testCallKey,
                events: await testEvents(storage.database),
                database: storage.database,
                server: {
                    ...(await testServerContext(Package.of(roomsPackage), spaceId)),
                    partition,
                    publisher: machine,
                    publisherAt: (address, packageId) => {
                        reached.push(`${address} ${packageId}`);

                        return root;
                    },
                },
            });
            const listed = present(store.subscriber?.subscriptions, "the store's subscriptions");
            const entries = await listed(Snapshot.live(storage.database));

            return entries.map((entry) => [
                entry.subscription.name,
                entry.subscription.scope,
                entry.publisher === machine
                    ? "machine"
                    : entry.publisher === root
                      ? "root"
                      : "other",
            ]);
        };

        // follow each link's chain, which at the universe has the access grants delegated into it
        expect([await subscriptions(spaceId), await subscriptions(ROOM), reached]).toEqual([
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
    },
);

test.each(TEST_DIALECTS)(
    "replicate the rooms from the universe into the space's root, and leave a room's own row to its partition's chain, on %s",
    async (dialect) => {
        // store the rooms' seats, replicating the rooms
        const storage = await TestDatabase.create(dialect, seatsDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);

        // list the types the universe replica of the root and of a room's partition request
        const idles = { machine: idle("machine"), root: idle("root") };
        const universeRows = async (partition?: string) => {
            const store = new ObjectStore({
                servers: [defineServer(seatsPackage, {})],
                chain: async () => ({
                    kind: "follower",
                    parent: partition === undefined ? Scope.universe.id : spaceId,
                }),
                clock: Clock.system,
                origin: { package: Package.of(seatsPackage) },
                callKey: testCallKey,
                events: await testEvents(storage.database),
                database: storage.database,
                server: {
                    ...(await testServerContext(Package.of(seatsPackage), spaceId)),
                    ...(partition === undefined ? {} : { partition }),
                    publisher: idles.machine,
                    publisherAt: () => idles.root,
                },
            });

            const listed = await store.sink.chains(
                partition ?? spaceId,
                Snapshot.live(storage.database),
            );

            return listed
                .filter((entry) => entry.subscription.publication === QUERIES_PUBLICATION)
                .map((entry) => entry.subscription.parameters["queries"]);
        };

        // query every room at the root, and leave the rooms to a room's partition chain
        expect([await universeRows(), await universeRows(ROOM)]).toEqual([
            [{ room: { object: room.name, scopes: "every" } }],
            [],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "keep a home's chain subscriptions through an install, and change a follower's own scope by the types it now homes, on %s",
    async (dialect) => {
        // store the space's cards and pins, with pins installed later
        const pin = defineObject({
            name: "pin",
            plural: "pins",
            scope: space,
            fields: { owner: field.reference(principal.user).caller() },
            permissions: { read: relation("owner"), write: relation("owner") },
        });
        const declared = defineDatabase({
            name: "main",
            tables: [...card.tables, ...pin.tables, ...callTables],
        });
        const storage = await TestDatabase.create(dialect, declared, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const idles = { machine: idle("machine"), root: idle("root") };
        const chains = async (types: readonly ObjectType[], kind: "home" | "follower") => {
            const store = new ObjectStore({
                servers: serversOf(types),
                chain: async () => ({ kind, parent: Scope.universe.id }),
                clock: Clock.system,
                origin: { package: card.package },
                callKey: testCallKey,
                events: await testEvents(storage.database),
                database: storage.database,
                server: {
                    ...(await testServerContext(card.package, spaceId)),
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

        // list the chain subscriptions before and after installing pins, for a home and a follower
        const home = [await chains([card], "home"), await chains([card, pin], "home")];
        const follower = [await chains([card], "follower"), await chains([card, pin], "follower")];
        const changed = nth(follower, 1).filter(
            (entry) =>
                !nth(follower, 0).some(
                    (before) => JSON.stringify(before) === JSON.stringify(entry),
                ),
        );

        // keep the home's, and change only the follower's own scope by the types it homes
        expect({ installed: nth(home, 1), changed }).toEqual({
            installed: nth(home, 0),
            changed: [
                [
                    "chain",
                    spaceId,
                    {
                        scopes: [spaceId, Scope.universe.id],
                        local: [card.typeReference, pin.typeReference],
                        types: [],
                        via: [],
                        between: [],
                    },
                ],
            ],
        });
    },
);

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
    ).toThrow(new TypeError("object room: share a partitioned scope through its relationships"));
});
