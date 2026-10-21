import { principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import type { DatabaseConnection } from "@destack/db";
import { ObjectStore } from "@destack/object/server";
import { testEvents, serversOf } from "@destack/object/test";
import { Package } from "@destack/package";
import { Identifier } from "@destack/schema";
import { testCallKey } from "@destack/object/test";
import { Scope } from "@destack/sync";
import { IdentityRegistry } from "../identity/registry.ts";
import { identity, machine, placement } from "../object/index.ts";
import accountPackage from "../package.ts";
import { implementIdentity } from "../server/identity.ts";
import { implementOperation } from "../server/operation.ts";
import { TestDirectory } from "./directory.ts";
import type { Clock } from "@destack/environment";

/** The universe's identity registry as a test's machine serving its spaces calls it. */
export const TestIdentityRegistry = {
    /** Serve identities over an account database to one machine, placing the spaces it serves. */
    async open(database: DatabaseConnection, clock: Clock, machineId: Identifier<"machine">) {
        // store the machine, and let machines start and extend identities as the universe's machine role does
        const serving = principal.machine.reference(Scope.universe.id, machineId);
        await TestDirectory.publish(database, machineId, Scope.universe.id, null);
        const access = new AccessFixture(database);
        await access.replicateRelationship(
            Scope.universe.id,
            Scope.universe,
            "machine",
            principal.machine.reference("*", "*"),
            clock.now(),
        );
        await access.replicateRole(
            Scope.universe,
            {
                name: "machine",
                description: "Start and extend the identities of the spaces machines serve",
                permissions: [
                    identity.permission("read"),
                    identity.permission("start"),
                    identity.permission("extend"),
                ],
            },
            { ...Scope.universe, relation: "machine" },
            clock.now(),
        );

        // serve the identities as the machine
        const store = new ObjectStore({
            chain: async () => undefined,
            clock,
            principal: serving,
            servers: serversOf([implementIdentity(), implementOperation()]),
            policies: [placement, machine],
            database,
            callKey: testCallKey,
            events: await testEvents(database, [], clock),
            origin: { package: Package.of(accountPackage) },
        });

        return {
            /** The registry the machine publishes its spaces' identities in. */
            registry: IdentityRegistry.of(store),
            /** Place a new space on the machine, and return it. */
            place: async (): Promise<Identifier<"space">> => {
                const space = Identifier.create("space");
                await TestDirectory.place(database, {
                    id: space,
                    scope: Identifier.create("account"),
                    machine: machineId,
                    epoch: 1,
                });

                return space;
            },
        };
    },
};
