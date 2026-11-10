import { principal, universe } from "@destack/access";
import { TestAccess } from "@destack/access/test";
import { type DatabaseConnection, eq } from "@destack/db";
import { type ClaimPrimary, RemoteStore } from "@destack/object/server";
import { Identifier } from "@destack/schema";
import { serversOf, testCallKey, testEvents, TestObjectStore } from "@destack/object/test";
import { Scope, type Uplink } from "@destack/sync";
import { account } from "../object/account.ts";
import { residency } from "../object/residency.ts";
import { space } from "../object/space.ts";
import { claim } from "../object/claim.ts";
import { machine } from "../object/machine.ts";
import { implementPlacement } from "../server/placement.ts";
import { implementSpace } from "../server/space.ts";
import { assignment, placement, Placement } from "../object/placement.ts";
import { Package } from "@destack/package";
import { endpoint } from "../object/endpoint.ts";
import accountPackage from "../package.ts";
import { STACKS } from "./account.ts";
import type { Clock, Environment } from "@destack/environment";

/** The universe's claims, placements and machines as a test writes them. */
export const TestDirectory = {
    /** Serve the universe's claims over a test's account database, claimed as the universe. */
    async claims(database: DatabaseConnection, environment: Environment): Promise<ClaimPrimary> {
        // bind the universe as its owner once
        const { clock } = environment;
        const owner = universe.reference(Scope.universe.id, Scope.universe.id);
        const access = new TestAccess(database, environment, Scope.universe.id);
        await access.bindOwner(Scope.universe, owner, clock.now());

        // serve the claims over their table, acting as the store's principal
        const store = TestObjectStore.of({
            chain: async () => undefined,
            environment,
            principal: owner,
            servers: serversOf([claim]),
            policies: [placement, account, residency],
            database,
            claims: { object: claim },
            callKey: testCallKey,
            events: await testEvents(database, environment),
            origin: { package: Package.of(accountPackage) },
        });

        return { object: claim, home: RemoteStore.served(store, owner) };
    },

    /** Run the placement calls a machine sends the account package over the universe's database. */
    async uplink(
        database: DatabaseConnection,
        sender: Identifier<"machine">,
        environment: Environment,
    ): Promise<Uplink> {
        // serve placements, assignments and spaces, acting as the store's principal
        const { clock } = environment;
        const store = TestObjectStore.of({
            chain: async () => undefined,
            environment,
            principal: universe.reference(Scope.universe.id, Scope.universe.id),
            servers: serversOf([implementPlacement(), assignment, implementSpace(STACKS)]),
            claims: { object: claim },
            database,
            callKey: testCallKey,
            events: await testEvents(database, environment),
            origin: { package: Package.of(accountPackage) },
        });

        // bind the universe as its owner, as the platform does, which relates runners to spaces
        const universal = universe.reference(Scope.universe.id, Scope.universe.id);
        const access = new TestAccess(database, environment, Scope.universe.id);
        await access.bindOwner(Scope.universe, universal, clock.now());

        // receive each mutation as the sending machine
        const signer = principal.machine.reference(Scope.universe.id, sender);

        return { ...store.uplink, receive: (mutation) => store.uplink.receive(mutation, signer) };
    },

    /** Place a space on a machine at an epoch, moving to a target by a transfer when given. */
    async place(
        database: DatabaseConnection,
        placed: Placement,
        clock: Clock,
        move: {
            /** The machine the space moves to. */
            readonly target: Identifier<"machine">;
            /** The transfer moving it. */
            readonly transferId: Identifier<"transfer">;
        } | null = null,
    ): Promise<void> {
        // write the space a placement belongs to once, then the placement or a replacement of it
        const now = clock.now();
        const stored = {
            id: placed.space,
            scope: placed.scope,
            name: `placed-${placed.space.slice(-12)}`,
            stack: STACKS.space,
            createdAt: now,
            updatedAt: now,
        };
        const fields = {
            scope: placed.scope,
            machineId: placed.machine,
            epoch: placed.epoch,
            targetId: move?.target ?? null,
            transferId: move?.transferId ?? null,
            schedule: {
                kind: "stay",
                condition: { status: "true", reason: "Unset", message: "placed by the test" },
            } as const,
        };
        await database.transaction(async (transaction) => {
            await transaction.insert(space.table).values(stored).onConflictDoNothing();
            await transaction
                .insert(placement.table)
                .values({
                    id: Identifier.derive("placement", Identifier.uuid(placed.space)),
                    spaceId: placed.space,
                    ...fields,
                    createdAt: now,
                    updatedAt: now,
                })
                .onConflictDoUpdate({ target: placement.table.spaceId, set: fields });
        });
    },

    /** Store a machine answering at a URL, or answering nowhere. */
    async publish(
        database: DatabaseConnection,
        id: Identifier<"machine">,
        scope: string,
        url: string | null,
        clock: Clock,
    ): Promise<void> {
        // store a personal machine
        const now = clock.now();
        const times = { createdAt: now, updatedAt: now };
        await database.transaction(
            async (transaction) =>
                await transaction
                    .insert(machine.table)
                    .values({
                        id,
                        scope,
                        ...times,
                        name: id.slice(-8),
                        kind: "personal",
                        platform: "macOS",
                        architecture: "arm64",
                        model: "Mac",
                    })
                    .onConflictDoNothing(),
        );

        // replace its endpoint, or withdraw it
        await database.transaction(
            async (transaction) =>
                await transaction.delete(endpoint.table).where(eq(endpoint.table.parentId, id)),
        );
        if (url !== null) {
            await database.transaction(
                async (transaction) =>
                    await transaction.insert(endpoint.table).values({
                        id: Identifier.derive("endpoint", Identifier.uuid(id)),
                        parentId: id,
                        scope,
                        url,
                        relay: null,
                        ...times,
                    }),
            );
        }
    },
};
