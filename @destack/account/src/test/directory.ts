import { TestEntitlement } from "./entitlement.ts";
import { accessRelationship, Caller, principal, Relationship, universe } from "@destack/access";
import { TestAccess } from "@destack/access/test";
import { MemoryBlobStorage } from "@destack/resource";
import { IdentityRegistry } from "../identity/registry.ts";
import { identity } from "../object/identity.ts";
import { implementIdentity } from "../server/identity.ts";
import { implementOperation } from "../server/operation.ts";
import { and, eq } from "@destack/db";
import type { DatabaseConnection } from "@destack/db/database";
import { relationship } from "@destack/object";
import { type ClaimPrimary, ObjectStore, RemoteStore } from "@destack/object/server";
import { Identifier } from "@destack/schema";
import { serversOf, testCallKey, testEvents, TestObjectStore } from "@destack/object/test";
import { Scope } from "@destack/sync";
import type { ObjectRemote } from "@destack/object/client";
import { account } from "../object/account.ts";
import { region } from "../object/region.ts";
import { space } from "../object/space.ts";
import { claim } from "../object/claim.ts";
import { machine } from "../object/machine.ts";
import { implementPlacement } from "../server/placement.ts";
import { implementSpace } from "../server/space.ts";
import { assignment, placement, Placement } from "../object/placement.ts";
import { PackageVersion } from "@destack/package";
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
            policies: [placement, account, region],
            database,
            claims: { object: claim },
            callKey: testCallKey,
            events: await testEvents(database, environment),
            origin: { package: PackageVersion.of(accountPackage) },
        });

        return { object: claim, home: RemoteStore.served(store, owner) };
    },

    /** Run the placement calls a machine sends the account package over the universe's database. */
    async remote(
        database: DatabaseConnection,
        sender: Identifier<"machine">,
        environment: Environment,
    ): Promise<ObjectRemote> {
        // serve placements, assignments and spaces under plans allowing nothing, acting as the store's principal
        const { clock } = environment;
        const entitlement = new TestEntitlement();
        const store = TestObjectStore.of({
            chain: async () => undefined,
            environment,
            principal: universe.reference(Scope.universe.id, Scope.universe.id),
            servers: serversOf([
                implementPlacement((accountId) => entitlement.of(accountId), []),
                assignment,
                implementSpace(STACKS, (accountId) => entitlement.of(accountId)),
                relationship,
            ]),
            claims: { object: claim },
            database,
            callKey: testCallKey,
            events: await testEvents(database, environment),
            origin: { package: PackageVersion.of(accountPackage) },
        });

        // bind the universe as its owner, as the platform does, which relates runners to spaces
        const universal = universe.reference(Scope.universe.id, Scope.universe.id);
        const access = new TestAccess(database, environment, Scope.universe.id);
        await access.bindOwner(Scope.universe, universal, clock.now());

        // take each push, call and event as its own signer, else as the sending machine
        const signer = principal.machine.reference(Scope.universe.id, sender);
        const { remote } = store;

        return {
            ...remote,
            push: (home, push, signal) =>
                remote.push(home, { ...push, signer: push.signer ?? Caller.of(signer) }, signal),
            call: (scope, call, given) => remote.call(scope, call, given ?? Caller.of(signer)),
            broadcast: (scope, object, event, given) =>
                remote.broadcast(scope, object, event, given ?? Caller.of(signer)),
        };
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
            // store the space once, then its placement
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

            // end any other machine's serving of the space
            const object = space.reference(placed.scope, placed.space);
            await transaction
                .delete(accessRelationship)
                .where(and(Relationship.on(object), eq(accessRelationship.relation, "machine")));

            // let the placed machine serve the space, as the placement relates it
            const serving = {
                id: Identifier.derive("relationship", Identifier.uuid(placed.space)),
                object,
                relation: "machine",
                subject: principal.machine.reference(Scope.universe.id, placed.machine),
                createdAt: now,
                expiresAt: null,
            };
            const scope = await Relationship.scope(transaction.snapshot(), serving);
            await transaction
                .insert(accessRelationship)
                .values(Relationship.encode(serving, scope));
        });
    },

    /** Store a machine answering at a URL, or answering nowhere. */
    async publish(
        database: DatabaseConnection,
        id: Identifier<"machine">,
        scope: Identifier<"account">,
        url: string | null,
        clock: Clock,
    ): Promise<void> {
        // store a self-hosted machine
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
                        kind: "self",
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

    /** Serve the identity registry to one machine of an account over the directory's database. */
    async registry(
        database: DatabaseConnection,
        environment: Environment,
        machineId: Identifier<"machine">,
        accountId: Identifier<"account">,
    ): Promise<IdentityRegistry> {
        // store the machine, and let machines start and extend identities as the universe's machine role does
        const { clock } = environment;
        const serving = principal.machine.reference(Scope.universe.id, machineId);
        await TestDirectory.publish(database, machineId, accountId, null, clock);
        const access = new TestAccess(database, environment);
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

        // serve the identities as the machine, the account package as the universe's own
        const store = new ObjectStore({
            blobs: new MemoryBlobStorage(environment.crypto),
            chain: async () => undefined,
            environment,
            principal: serving,
            servers: serversOf([implementIdentity(), implementOperation()]),
            policies: [placement, machine],
            database,
            callKey: testCallKey,
            events: await testEvents(database, environment),
            origin: { package: PackageVersion.of(accountPackage) },
        });

        return IdentityRegistry.of(store, serving);
    },
};
