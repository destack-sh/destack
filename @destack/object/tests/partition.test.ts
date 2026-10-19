import { Package } from "@destack/package";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { none, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";
import { definePackage } from "@destack/runtime";
import { testServerContext } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import type { Uplink } from "@destack/sync";
import { testCallKey } from "@destack/runtime/test";
import { defineObject, field } from "../src/index.ts";
import { ShareableMethod } from "../src/method/shareable.ts";
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

/** An uplink streaming and receiving nothing, named by who it stands for. */
function idle(name: string): Uplink {
    return {
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
                chain: async () => ({ kind: "follower" }),
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
            const entries = await listed();

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

        // follow each link's chain and the access grants delegated into it from the universe
        expect([await subscriptions(spaceId), await subscriptions(ROOM), reached]).toEqual([
            [
                [`chain:${spaceId}`, spaceId, "machine"],
                [`chain:${spaceId}`, "universe", "machine"],
                [spaceId, "universe", "machine"],
            ],
            [
                [`chain:${ROOM}`, ROOM, "root"],
                [`chain:${ROOM}`, spaceId, "root"],
                [`chain:${ROOM}`, "universe", "root"],
                [ROOM, "universe", "root"],
            ],
            [`fixture--${spaceId} ${roomsPackage.id}`],
        ]);
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
        ).describe(),
    ).toThrow(new TypeError("object room: share a partitioned scope through its relationships"));
});
