import { none, universe } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import type { DatabaseConnection } from "@destack/db";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf, TestObjectStore } from "../../src/test/index.ts";
import { Scope } from "@destack/sync";
import { type ClaimHome, RemoteStore } from "../../src/server/index.ts";
import { claimMethods, claimTable, defineObject } from "../../src/index.ts";
import type { Environment } from "@destack/environment";

/** The universe's claimed names, which only its principal changes in these tests. */
export const claim = defineObject(
    {
        name: "claim",
        plural: "claims",
        mapping: {
            table: claimTable,
            id: "id",
            scope: "scope",
            attributes: {},
            relations: {},
        },
        scope: "universe",
        permissions: { read: none(), claim: none(), resolve: none() },
    },
    () => claimMethods,
);

/** Serve the universe's claims over its database, called as the universe's principal. */
export async function serveClaims(
    database: DatabaseConnection,
    environment: Environment,
): Promise<ClaimHome> {
    // bind the universe as its owner
    const owner = universe.reference(Scope.universe.id, Scope.universe.id);
    await new AccessFixture(database, environment).replicateOwner(Scope.universe, owner);

    // serve the claims, claiming the names of nothing it writes itself
    const store = await TestObjectStore.receive(Scope.universe, {
        chain: async () => undefined,
        environment,
        principal: owner,
        servers: serversOf([claim]),
        database,
        claims: { object: claim },
        callKey: testCallKey,
        events: await testEvents(database, environment),
        origin: { package: claim.package },
    });

    return { object: claim, home: RemoteStore.served(store, owner) };
}
