import { principal, Role, universe } from "@destack/access";
import { and, type DatabaseConnection, eq, Transaction } from "@destack/db";
import { type ClaimHome, ObjectStore, RemoteStore } from "@destack/object/server";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, Clock, present } from "@destack/schema";
import { createClient, type ClientOf } from "@destack/runtime/client";
import { Health } from "@destack/runtime/health";
import { Server } from "@destack/runtime/server";
import { testCallKey } from "@destack/runtime/test";
import { testEvents } from "@destack/object/test";
import { Scope, type Uplink } from "@destack/sync";
import { account } from "../object/account.ts";
import { residency } from "../object/residency.ts";
import { claim } from "../object/claim.ts";
import { machine } from "../object/machine.ts";
import { implementPlacement } from "../server/placement.ts";
import { implementSpace } from "../server/space.ts";
import { assignment, placement, type Placement } from "../object/placement.ts";
import { Package } from "@destack/package";
import { endpoint } from "../object/endpoint.ts";
import accountPackage from "../package.ts";
import { STACKS } from "./account.ts";

/** The header with the machine a test's account package call comes from. */
const MACHINE_HEADER = "x-machine";

/** How long a test's account package drains its requests, in milliseconds. */
const DRAIN_MILLISECONDS = 1000;

/** How long a test machine's credential lasts: a test's length with room to spare. */
const CREDENTIAL_MILLISECONDS = 60 * 60 * 1000;

/** The universe's placements, machines and assignments as a test writes them. */
export const TestDirectory = {
    /** Serve the universe's claims over a test's account database, claimed as the universe. */
    async claims(database: DatabaseConnection, clock: Clock): Promise<ClaimHome> {
        // own the universe as its principal once
        const owner = universe.reference(Scope.universe.id, Scope.universe.id);
        await Transaction.run(database, (transaction) =>
            Role.bindOwner(transaction, Scope.universe, owner, clock.now()),
        );

        // serve the claims over their table
        const store = new ObjectStore({
            chain: async () => undefined,
            clock,
            principal: owner,
            objects: [claim],
            policies: [placement, account, residency],
            database,
            claims: { object: claim },
            callKey: testCallKey,
            events: await testEvents(database, [], clock),
            origin: { package: Package.of(accountPackage) },
        });

        return { object: claim, home: RemoteStore.served(store, owner) };
    },

    /** Serve the account objects machines read over a test's account database, as the universe. */
    async serve(database: DatabaseConnection, clock: Clock): Promise<Server> {
        // serve placements and assignments as the universe
        const store = new ObjectStore({
            chain: async () => undefined,
            clock,
            principal: universe.reference(Scope.universe.id, Scope.universe.id),
            objects: [implementPlacement(), assignment],
            database,
            callKey: testCallKey,
            events: await testEvents(database, [], clock),
            origin: { package: Package.of(accountPackage) },
        });
        const implementation = store.implement(accountPackage, store.routed, []);

        // act as the machine in each request's header
        const audience = accountPackage.id;

        return Server.start({
            ...implementation,
            clock,
            resources: new ResourceContext(),
            health: new Health("universe"),
            drainTimeout: DRAIN_MILLISECONDS,
            authorizeMachine: async () => {},
            authenticate: async (request) => {
                // read the machine and present a fixture credential for it
                const id = present(request.headers.get(MACHINE_HEADER), "the machine header");
                const subject = principal.machine.reference(Scope.universe.id, id);
                const now = clock.now();

                return {
                    subject,
                    subjects: [subject],
                    credential: {
                        kind: "fixture",
                        id,
                        audience,
                        verifiedAt: now,
                        expiresAt: now + CREDENTIAL_MILLISECONDS,
                    },
                };
            },
        });
    },

    /** Call a test's account package as a machine. */
    client(
        server: Pick<Server, "fetch">,
        machineId: Identifier<"machine">,
    ): ClientOf<typeof accountPackage> {
        return createClient(accountPackage, {
            url: "http://accounts.test",
            headers: { [MACHINE_HEADER]: machineId },
            fetch: (request) => server.fetch(request),
        });
    },

    /** Run the placement calls a machine sends the account package over the universe's database. */
    async uplink(database: DatabaseConnection, sender: Identifier<"machine">): Promise<Uplink> {
        // serve placements, assignments and spaces as the universe
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: universe.reference(Scope.universe.id, Scope.universe.id),
            objects: [implementPlacement(), assignment, implementSpace(STACKS)],
            claims: { object: claim },
            database,
            callKey: testCallKey,
            events: await testEvents(database, [], Clock.system),
            origin: { package: Package.of(accountPackage) },
        });

        // own the universe as the universe, as the platform does, which relates runners to spaces
        const universal = universe.reference(Scope.universe.id, Scope.universe.id);
        await Transaction.run(database, (transaction) =>
            Role.bindOwner(transaction, Scope.universe, universal, Date.now()),
        );

        // receive each mutation as the sending machine
        const signer = principal.machine.reference(Scope.universe.id, sender);

        return { ...store.uplink, receive: (mutation) => store.uplink.receive(mutation, signer) };
    },

    /** Place a space on a machine at an epoch, moving to a target by a transfer when given. */
    async place(
        database: DatabaseConnection,
        placed: Placement,
        move: {
            /** The machine the space moves to. */
            readonly target: Identifier<"machine">;
            /** The transfer moving it. */
            readonly transferId: Identifier<"transfer">;
        } | null = null,
    ): Promise<void> {
        // write the placement, or replace the one the space has
        const now = Date.now();
        const fields = {
            scope: placed.scope,
            machineId: placed.machine,
            epoch: placed.epoch,
            targetId: move?.target ?? null,
            transferId: move?.transferId ?? null,
        };
        await database
            .insert(placement.table)
            .values({ id: placed.id, ...fields, createdAt: now, updatedAt: now })
            .onConflictDoUpdate({ target: placement.table.id, set: fields });
    },

    /** Keep a machine answering at a URL, or answering nowhere. */
    async publish(
        database: DatabaseConnection,
        id: Identifier<"machine">,
        scope: string,
        url: string | null,
    ): Promise<void> {
        // keep a personal machine
        const now = Date.now();
        const times = { createdAt: now, updatedAt: now };
        await database
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
            .onConflictDoNothing();

        // replace its endpoint, or withdraw it
        await database.delete(endpoint.table).where(eq(endpoint.table.id, id));
        if (url !== null) {
            await database
                .insert(endpoint.table)
                .values({ id, parentId: id, scope, url, relay: null, ...times });
        }
    },

    /** Give a machine work in a space, or withdraw it. */
    async assign(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        worker: Identifier<"machine">,
        isAssigned: boolean,
    ): Promise<void> {
        // give the work once
        if (isAssigned) {
            const now = Date.now();
            await database
                .insert(assignment.table)
                .values({
                    id: Identifier.create("assignment"),
                    placementId: spaceId,
                    machineId: worker,
                    scope: Scope.universe.id,
                    createdAt: now,
                    updatedAt: now,
                })
                .onConflictDoNothing();
        }
        // withdraw it
        else {
            await database
                .delete(assignment.table)
                .where(
                    and(
                        eq(assignment.table.placementId, spaceId),
                        eq(assignment.table.machineId, worker),
                    ),
                );
        }
    },
};
