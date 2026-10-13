import { none, universe } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import type { DatabaseConnection } from "@destack/db";
import type { Clock } from "@destack/schema";
import { testCallKey } from "@destack/runtime/test";
import { testEvents } from "../../src/test/index.ts";
import { Scope } from "@destack/sync";
import { type ClaimHome, ObjectStore, RemoteStore } from "../../src/server/index.ts";
import { Claim, claimMethods, claimTable, defineObject, INTRINSIC } from "../../src/index.ts";

/** The universe's claimed names, which only its own principal changes in these tests. */
export const claim = defineObject({
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
})
    .methods(() => claimMethods)
    .implement({
        methods: {
            reserve: async (call) => {
                const { request, ...claimant } = call.input;
                const { database } = call.requireTransaction();

                return { refused: await Claim.reserve(database, claimant, request, call.now) };
            },
            confirm: async (call) => {
                const { request, ...claimant } = call.input;
                await Claim.confirm(call.requireTransaction().database, claimant, request);

                return {};
            },
            release: async (call) => {
                const { request, ...object } = call.input;
                await Claim.release(call.requireTransaction().database, object, request);

                return {};
            },
            replace: async (call) => ({
                refused: await Claim.replace(call.requireTransaction().database, call.input),
            }),
            owner: async (call) => (await Claim.owner(call.read(claimTable), call.input)) ?? null,
            expired: (call) => Claim.expired(call.read(claimTable), call.input.indexes, call.now),
        },
    });

/** Serve the universe's claims over its database, called as the universe's own principal. */
export async function serveClaims(database: DatabaseConnection, clock: Clock): Promise<ClaimHome> {
    // own the universe as its principal
    const owner = universe.reference(Scope.universe.id, Scope.universe.id);
    await new AccessFixture(database).copyOwner(Scope.universe, owner);

    // serve the claims, claiming the names of nothing it writes itself
    const store = new ObjectStore({
        clock,
        principal: owner,
        objects: [claim],
        database,
        claims: { object: claim },
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: claim.package },
    });

    return { object: claim, home: RemoteStore.served(store, owner) };
}
