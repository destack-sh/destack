import { none, universe } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import type { DatabaseConnection } from "@destack/db";
import { testCallKey } from "@destack/runtime/test";
import { testEvents, serversOf } from "../../src/test/index.ts";
import { Scope } from "@destack/sync";
import { type ClaimHome, ObjectStore, RemoteStore } from "../../src/server/index.ts";
import { claimMethods, claimTable, defineObject, INTRINSIC } from "../../src/index.ts";
import type { Clock } from "@destack/environment";

/** The universe's claimed names, which only its own principal changes in these tests. */
export const claim = defineObject(
    {
        name: "claim",
        plural: "claims",
        [INTRINSIC]: {
            table: claimTable,
            mapping: {
                table: claimTable,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {},
            },
        },
        scope: "universe",
        permissions: { read: none(), claim: none(), resolve: none() },
    },
    () => claimMethods,
);

/** Serve the universe's claims over its database, called as the universe's own principal. */
export async function serveClaims(database: DatabaseConnection, clock: Clock): Promise<ClaimHome> {
    // own the universe as its principal
    const owner = universe.reference(Scope.universe.id, Scope.universe.id);
    await new AccessFixture(database).bindOwner(Scope.universe, owner);

    // serve the claims, claiming the names of nothing it writes itself
    const store = new ObjectStore({
        chain: async () => undefined,
        clock,
        principal: owner,
        servers: serversOf([claim]),
        database,
        claims: { object: claim },
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: claim.package },
    });

    return { object: claim, home: RemoteStore.served(store, owner) };
}
