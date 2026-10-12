import { expect, onTestFinished, refusal, test } from "@destack/test";
import { none, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { journal } from "@destack/audit/stack";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { present, schema, Clock } from "@destack/schema";
import { defineService } from "@destack/service";
import { testInstallation } from "@destack/service/test";
import type { Uplink } from "@destack/sync";
import { testCallKey } from "@destack/service/test";
import { defineObject, field } from "../src/index.ts";
import { ObjectExtension, ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { request } from "./fixture/request.ts";
import { userContext } from "./fixture/user.ts";

/** The space whose partition the cards live in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000a1");

/** Cards their owners pin in a space. */
const card = defineObject({
    name: "card",
    plural: "cards",
    scope: space,
    fields: { owner: field.reference(principal.user).caller() },
    permissions: { read: relation("owner"), write: relation("owner") },
}).methods((method) => ({ create: method.create("write", { fields: [] }) }));

/** A room inside the space, a partition of its own. */
const room = defineObject({
    name: "room",
    plural: "rooms",
    scope: space,
    isScope: true,
    partition: true,
    fields: {},
    permissions: { read: none() },
}).methods((method) => ({ get: method.get("read") }));

/** The room the scenarios serve a partition of. */
const ROOM = "room-01996ab0-0000-7000-8000-0000000000a3";

/** The service of the space's rooms. */
const roomsService = defineService("rooms", { objects: { room } });

/** The database of the space's rooms. */
const roomsDatabase = defineDatabase({ name: "main", tables: [...room.tables, journal] });

/** An uplink streaming and receiving nothing, named by who it stands for. */
function idle(name: string): Uplink {
    return {
        stream: () => {
            throw new TypeError(`${name} streams no copies`);
        },
        receive: () => Promise.reject(new TypeError(`${name} receives no changes`)),
    };
}

/** The database of the space's cards. */
const cardsDatabase = defineDatabase({ name: "main", tables: [...card.tables, journal] });

test.each(TEST_DIALECTS)(
    "serve a call in the store's own partition or in any partition, and point one in another partition to the partition serving it on %s",
    async (dialect) => {
        // keep the space's cards, served by a store per partition it may serve
        const storage = await TestDatabase.create(dialect, cardsDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const serve = (partition?: string) =>
            new ObjectStore({
                clock: Clock.system,
                principal: spacePrincipal(spaceId),
                objects: { card },
                database: storage.database,
                callKey: testCallKey,
                origin: { package: card.package, service: "test" },
                ...(partition === undefined ? {} : { partition }),
            });
        const create = (store: ObjectStore) =>
            request(store, card, "create", { spaceId }, userContext("user-1", spaceId));

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
    "follow an installation's root chain from its machine, and a partition's chain below the root from the installation's root through its own package, one copy per link, on %s",
    async (dialect) => {
        // keep a room partitioning inside the space
        const storage = await TestDatabase.create(dialect, roomsDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        await new AccessFixture(storage.database).copyScope(room.reference(spaceId, ROOM), "own");

        // serve the installation at its root and in the room, recording the publishers it reaches
        const reached: string[] = [];
        const machine = idle("machine");
        const root = idle("root");
        const subscriptions = async (partition: string) => {
            const { store } = ObjectExtension.compose([ObjectExtension.of(roomsService, [])], {
                clock: Clock.system,
                origin: { package: roomsService.package, service: roomsService.name },
                callKey: testCallKey,
                database: storage.database,
                installation: {
                    ...(await testInstallation(roomsService.package, spaceId)),
                    partition,
                    publisher: machine,
                    publisherAt: (address, packageId) => {
                        reached.push(`${address} ${packageId}`);

                        return root;
                    },
                },
            });
            const entries = await present(
                store.subscriber,
                "the store's subscriber",
            ).subscriptions();

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
            [
                `installation-019f5530-8000-7000-8000-00000000f001--${spaceId} ${roomsService.package.id}`,
            ],
        ]);
    },
);
