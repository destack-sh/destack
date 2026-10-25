import { principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import type { DatabaseConnection } from "@destack/db";
import { Installation } from "@destack/object";
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
import type { Environment } from "@destack/environment";

/** The universe's identity registry as a test's machine serving its spaces calls it. */
export const TestIdentityRegistry = {
    /** Serve identities over an account database to one machine, placing the spaces it serves. */
    async open(
        database: DatabaseConnection,
        environment: Environment,
        machineId: Identifier<"machine">,
    ) {
        // store the machine, and let machines start and extend identities as the universe's machine role does
        const { clock } = environment;
        const serving = principal.machine.reference(Scope.universe.id, machineId);
        await TestDirectory.publish(database, machineId, Scope.universe.id, null, clock);
        const access = new AccessFixture(database, environment);
        await access.replicateRelationship(
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

        // serve the identities as the machine, the account package as its base installation at the universe
        const base = await Installation.base(Scope.universe, accountPackage, environment.crypto);
        const store = new ObjectStore({
            chain: async () => undefined,
            environment,
            principal: serving,
            servers: serversOf([implementIdentity(), implementOperation()]).map((server) =>
                server.at(base),
            ),
            policies: [placement, machine],
            database,
            callKey: testCallKey,
            events: await testEvents(database, environment),
            origin: { package: Package.of(accountPackage) },
        });

        return {
            /** The registry the machine publishes its spaces' identities in. */
            registry: IdentityRegistry.of(store, serving),
            /** Place a new space on the machine, and return it. */
            place: async (): Promise<Identifier<"space">> => {
                const space = Identifier.create("space", environment);
                await TestDirectory.place(
                    database,
                    {
                        id: space,
                        scope: Identifier.create("account", environment),
                        machine: machineId,
                        epoch: 1,
                    },
                    clock,
                );

                return space;
            },
        };
    },
};
