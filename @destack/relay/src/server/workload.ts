import { principal } from "@destack/access";
import { defineDatabase, Snapshot } from "@destack/db";
import { MemoryBlobStorage } from "@destack/resource";
import { DOMAINS } from "@destack/account/address";
import {
    directoryShape,
    identityShape,
    machine,
    type ResidencyCode,
    residentsShape,
} from "@destack/account/object";
import { Egress, Follow, PackageMount, tablesOf } from "@destack/object";
import objectPackage from "@destack/object/package";
import { implementPackage, ObjectStore, type PackageHandler } from "@destack/object/server";
import {
    defineWorkload,
    ForwardedRequest,
    ROUTE_PATH,
    type InstalledServerContext,
    type WorkloadContext,
    type WorkloadImplementation,
} from "@destack/object/workload";
import { Failure } from "@destack/error";
import { present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { issuer, origin, residency } from "../declare/setting.ts";
import { RelayError } from "../error/index.ts";
import relayPackage from "../package.ts";
import { LapseController } from "./lapse-controller.ts";
import { NameController } from "./name-controller.ts";
import { Relay } from "./relay.ts";
import { TunnelToken } from "./token.ts";
import {
    HANDSHAKE_HEADER,
    Handshake,
    Tunnel,
    TunnelCloseReason,
    TunnelStanding,
    TUNNEL_PATH,
    TUNNELS_PATH,
} from "./tunnel.ts";

/** The universe's identities, whose keys verify the machines' tunnel tokens. */
const IDENTITY_FOLLOW = Follow.of(identityShape, { keys: [{ key: { scope: Scope.universe.id } }] });

/** The universe's directory, which names resolve through. */
const DIRECTORY_FOLLOW = Follow.of(directoryShape, {
    keys: [{ key: { scope: Scope.universe.id } }],
});

/** Follow the accounts of the residency a relay serves, whose machines alone tunnel to it. */
function residentsFollow(code: ResidencyCode) {
    return Follow.of(residentsShape, {
        keys: [{ key: { scope: Scope.universe.id, residency: code } }],
    });
}

/** The replica of the universe's directory, identities and residents in a relay's memory. */
export const relayDatabase = defineDatabase({
    name: "relay",
    ...tablesOf({
        packages: [objectPackage],
        follows: [DIRECTORY_FOLLOW, IDENTITY_FOLLOW, Follow.of(residentsShape, { keys: [] })],
        tables: [],
    }),
});

/** The relay's workload: the router of names at its root, a machine's tunnel in a partition. */
export const relayWorkload = defineWorkload({
    name: "relay",
    start: (context) => {
        // route names at the root, and run a machine's tunnel in its partition
        const server = present(context.server, "the relay installation's server context");

        return server.partition === undefined
            ? serveRoot(context, server)
            : servePartition(context, server, server.partition);
    },
});

/** Serve the root: the router of names over the directory's replica, and the names' controller. */
async function serveRoot(
    context: WorkloadContext,
    server: InstalledServerContext,
): Promise<WorkloadImplementation> {
    // answer the partitions' reads of their machines' standing, refusing every other call at the root
    const answer = async (request: Request): Promise<Response> => {
        // require a read of a machine's standing
        const [, machineId, verb] = new URL(request.url).pathname.split("/");
        if (verb !== "standing") {
            throw RelayError.of(
                "TUNNEL_CALL_UNKNOWN",
                `the relay's root answers no call ${verb ?? ""}`,
            );
        }

        // answer whether the machine stands, and its name
        return Response.json(await relay.standing(schema.identifier("machine").parse(machineId)));
    };

    // follow the directory and the identities into memory
    const residencyId = residency.read(server.settings);
    const residents = residentsFollow(residencyId);
    const follows = [DIRECTORY_FOLLOW, IDENTITY_FOLLOW, residents];
    const store = await openReplica(context, server, follows, answer);
    const relay = new Relay({
        origin: origin.read(server.settings),
        issuer: issuer.read(server.settings),
        residency: residencyId,
        domains: DOMAINS,
        itself: server.at(Egress.address(server.name, server.scope)),
        view: Snapshot.live(store.database),
        fetch: (request) => server.fetch(request),
        environment: context.environment,
        synced: async () => {
            // wait until the directory's and the residents' replicas synced
            const isSynced = async () => {
                const synced = await Promise.all([
                    DIRECTORY_FOLLOW.shape.isSynced(store.database, { scope: Scope.universe.id }),
                    residents.shape.isSynced(store.database, {
                        scope: Scope.universe.id,
                        residency: residencyId,
                    }),
                ]);

                return synced.every(Boolean);
            };
            await store.database.log.until(isSynced, context.signal);
        },
    });

    // hand the route's requests to the router, and reconcile the tunnels' names
    const handler = store.package(relayPackage);
    const served: PackageHandler = {
        ...handler,
        controllers: [...(handler.controllers ?? []), new NameController(relay)],
        fetch: async (request, requestContext) =>
            new URL(request.url).pathname === ROUTE_PATH
                ? relay.fetch(restore(request))
                : handler.fetch?.(request, requestContext),
    };

    return { packages: [served] };
}

/** Serve a machine's partition: its tunnel over the hibernated sockets, and their lapses. */
async function servePartition(
    context: WorkloadContext,
    server: InstalledServerContext,
    partition: string,
): Promise<WorkloadImplementation> {
    // answer the root's calls to the tunnel
    const answer = (request: Request) => tunnelCall(request, running);

    // follow the identities into memory, verifying the renewals by their keys
    const store = await openReplica(context, server, [IDENTITY_FOLLOW], answer);
    const root = server.at(Egress.address(server.name, server.scope));
    const running: Tunnel = new Tunnel(Tunnel.machine(partition), {
        hibernation: present(context.hibernation, "the hibernation of the tunnel's sockets"),
        url: new URL(TUNNEL_PATH, origin.read(server.settings)).href,
        tokens: new TunnelToken(
            issuer.read(server.settings),
            Snapshot.live(store.database),
            context.environment,
        ),
        identities: async () => {
            // wait until the replica of the universe's identities synced
            const isSynced = () =>
                IDENTITY_FOLLOW.shape.isSynced(store.database, { scope: Scope.universe.id });
            await store.database.log.until(isSynced, context.signal);
        },
        standing: async () => {
            // read whether the machine stands at the root
            const machineId = Tunnel.machine(partition);
            const path = `${PackageMount.path(relayPackage.id)}${TUNNELS_PATH}`;
            const response = await root.fetch(
                new Request(`${root.url}${path}/${machineId}/standing`),
            );
            if (!response.ok) {
                throw Failure.error(await Failure.read(response));
            }

            return TunnelStanding.parse(await response.json());
        },
        changed: () => lapses.changed(),
        publish: async (account, lapsesAt) => {
            // publish the tunnel at the universe as the relay installation, at the relay's origin
            const relayed = machine.call("relay", {
                ...machine.target({ scope: account, id: Tunnel.machine(partition) }),
                url: new URL(origin.read(server.settings)).origin,
                lapsesAt,
            });
            await server.publisher.receive(
                { id: RequestId.create(context.environment), calls: [relayed] },
                principal.installation.reference(server.scope, server.installation),
            );
        },
        environment: context.environment,
    });
    const lapses = new LapseController(running);

    // serve the tunnel's sockets, closing them as they lapse on the host's alarm
    const handler = store.package(relayPackage);
    const served: PackageHandler = {
        ...handler,
        controllers: [...(handler.controllers ?? []), lapses],
    };

    return { packages: [served], sockets: running };
}

/** Open a store following some of the universe's rows into the instance's memory. */
async function openReplica(
    context: WorkloadContext,
    server: InstalledServerContext,
    follows: readonly ReturnType<typeof Follow.of>[],
    tunnels: (request: Request) => Promise<Response>,
): Promise<ObjectStore> {
    // open the replica in a memory database, closed with the start
    const ephemeral = present(context.ephemeral, "the memory of the relay's instance");
    const database = await ephemeral.open(relayDatabase.tables);
    await database.migrate(relayDatabase.tables);
    context.defer(() => database.close());

    // follow the rows through the home space's replicas, served at the relay installation
    return new ObjectStore({
        blobs: new MemoryBlobStorage(context.environment.crypto),
        servers: [implementPackage(relayPackage, { follows, adapters: { tunnels } }).at(server)],
        chain: async () => undefined,
        principal: principal.installation.reference(server.scope, server.installation),
        publisher: () => server.publisher,
        database,
        environment: context.environment,
        origin: { package: relayPackage },
    });
}

/** Answer one call of the root to a machine's tunnel. */
async function tunnelCall(request: Request, running: Tunnel): Promise<Response> {
    // require the root to address the partition's own machine
    const [, addressed, verb] = new URL(request.url).pathname.split("/");
    if (addressed !== running.machineId) {
        throw new TypeError(`tunnel of ${running.machineId} got a call for ${addressed ?? "none"}`);
    }

    // accept a machine's WebSocket after its handshake for the same machine
    if (verb === "open") {
        const header = request.headers.get(HANDSHAKE_HEADER);
        if (header === null) {
            throw RelayError.of("HANDSHAKE_MISSING", "the tunnel's opening has no handshake");
        }
        const handshake = Handshake.parse(JSON.parse(header));
        if (handshake.machineId !== running.machineId) {
            throw new TypeError(
                `tunnel of ${running.machineId} got the handshake of ${handshake.machineId}`,
            );
        }

        return running.accept(handshake);
    }
    // forward a request through the tunnel under the host it asked for
    else if (verb === "forward") {
        return running.fetch(restore(request));
    }
    // tell the tunnel the machine's name
    else if (verb === "rename") {
        const { name } = schema
            .object({ name: schema.string().min(1) })
            .parse(await request.json());
        await running.rename(name);

        return new Response(null, { status: 204 });
    }
    // close the tunnel's sockets for a reason
    else if (verb === "close") {
        const { reason } = schema.object({ reason: TunnelCloseReason }).parse(await request.json());
        await running.closeAll(reason);

        return new Response(null, { status: 204 });
    }
    // refuse any other call
    else {
        throw RelayError.of("TUNNEL_CALL_UNKNOWN", `unknown tunnel call ${verb ?? ""}`);
    }
}

/** Restore the client's request a front door or the router handed on, with no header they added. */
function restore(request: Request): Request {
    const restored = ForwardedRequest.restore(request);
    if (restored === undefined) {
        throw RelayError.of("FORWARDED_HEAD_MISSING", "the handed request has no client head");
    }

    return restored;
}
