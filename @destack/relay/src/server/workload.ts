import { TUNNEL_PATH } from "@destack/tunnel";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { principal } from "@destack/access";
import { copyRequest } from "@destack/environment";
import { type DatabaseSchema, Relations, TABLE } from "@destack/db";
import { MemoryBlobStorage } from "@destack/resource";
import { UniverseOrigin } from "@destack/account/address";
import { directoryShape, identityShape, regionShape } from "@destack/account/object";
import { Egress, type Follow, PackageMount, tablesOf, VERSION_HEADER } from "@destack/object";
import objectPackage from "@destack/object/package";
import { implementPackage, ObjectStore } from "@destack/object/server";
import {
    defineWorkload,
    ClientHead,
    type ServerContext,
    type WorkloadContext,
    type WorkloadImplementation,
} from "@destack/object/workload";
import { Failure } from "@destack/error";
import { present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { origin, region, universe } from "../declare/setting.ts";
import { RelayError } from "../error/index.ts";
import relayPackage from "../package.ts";
import { Relay } from "./relay.ts";
import { TunnelToken } from "./token.ts";
import {
    HANDSHAKE_HEADER,
    Handshake,
    Tunnel,
    TunnelCloseReason,
    TunnelStanding,
    TUNNELS_PATH,
} from "./tunnel.ts";

/** The universe's identities, whose keys verify the machines' tunnel tokens. */
const IDENTITY_FOLLOW = identityShape.at({ key: { scope: Scope.universe.id } });

/** The universe's directory, which names resolve through. */
const DIRECTORY_FOLLOW = directoryShape.at({ key: { scope: Scope.universe.id } });

/** The universe's regions, whose containment sets the machines' tunnel tokens carry. */
const REGION_FOLLOW = regionShape.at({ key: { scope: Scope.universe.id } });

/** The tables of a relay's memory replica: its own and the followed ones. */
const REPLICA_TABLES = tablesOf({
    packages: [objectPackage],
    follows: [DIRECTORY_FOLLOW, IDENTITY_FOLLOW, REGION_FOLLOW],
    tables: [],
});

/** A relay's memory replica, storing the followed tables as replicas without their foreign keys. */
const REPLICA: DatabaseSchema = {
    tables: [...REPLICA_TABLES.tables, ...(REPLICA_TABLES.replicated ?? [])],
    replicated: new Set((REPLICA_TABLES.replicated ?? []).map((table) => table[TABLE].sqlName)),
    relations: new Relations(),
};

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
    server: ServerContext,
): Promise<WorkloadImplementation> {
    // answer the partitions' reads of their machines' standing, refusing every other call at the root
    const answer = async (request: Request): Promise<Response> => {
        // require a read of a machine's standing
        const [, machineId, verb] = new URL(request.url).pathname.split("/");
        if (verb !== "standing") {
            throw RelayError.of(
                "TUNNEL_CALL_NOT_FOUND",
                `relay root call '${verb ?? ""}' not found`,
            );
        }

        // answer whether the machine stands, and its name
        return Response.json(await relay.standing(schema.identifier("machine").parse(machineId)));
    };

    // follow the directory and the identities into memory
    const code = region.read(server.settings);
    const address = UniverseOrigin.of(universe.read(server.settings));
    const follows = [DIRECTORY_FOLLOW, IDENTITY_FOLLOW, REGION_FOLLOW];
    const names = (request: Request) =>
        relay.fetch(copyRequest(restore(request), { signal: request.signal }));
    const store = await openReplica(context, server, follows, answer, names);
    const relay = new Relay({
        origin: origin.read(server.settings),
        universe: address,
        itself: itselfOf(server),
        view: store.database.snapshot(),
        fetch: (request) => server.fetch(request),
        environment: context.environment,
        region: code,
        synced: async () => {
            // wait until the directory's, the identities' and the regions' replicas synced
            const isSynced = async () => {
                const synced = await Promise.all([
                    DIRECTORY_FOLLOW.shape.isSynced(store.database, { scope: Scope.universe.id }),
                    IDENTITY_FOLLOW.shape.isSynced(store.database, { scope: Scope.universe.id }),
                    REGION_FOLLOW.shape.isSynced(store.database, { scope: Scope.universe.id }),
                ]);

                return synced.every(Boolean);
            };
            await store.database.log.waitUntil(isSynced, context.signal);
        },
    });

    // serve the package, and project the tunnels' names from the replica in the store's loop
    const control = () => [{ controllers: [], projections: [relay.projection()], cursors: [] }];

    return { packages: [store.package(relayPackage)], store, control };
}

/** Serve a machine's partition: its tunnel over the hibernated sockets. */
async function servePartition(
    context: WorkloadContext,
    server: ServerContext,
    partition: string,
): Promise<WorkloadImplementation> {
    // answer the root's calls to the tunnel
    const answer = (request: Request) => tunnelCall(request, running);

    // follow the identities and the regions into memory, verifying the renewals by them
    const follows = [IDENTITY_FOLLOW, REGION_FOLLOW];
    const store = await openReplica(context, server, follows, answer, refuseNames);
    const root = itselfOf(server);
    const running: Tunnel = new Tunnel(Tunnel.machine(partition), {
        hibernation: present(context.hibernation, "the hibernation of the tunnel's sockets"),
        url: new URL(TUNNEL_PATH, origin.read(server.settings)).href,
        tokens: new TunnelToken(
            UniverseOrigin.of(universe.read(server.settings)).issuer,
            region.read(server.settings),
            store.database.snapshot(),
            context.environment,
        ),
        synced: async () => {
            // wait until the replicas of the universe's identities and regions synced
            const isSynced = async () => {
                const synced = await Promise.all(
                    follows.map((follow) =>
                        follow.shape.isSynced(store.database, { scope: Scope.universe.id }),
                    ),
                );

                return synced.every(Boolean);
            };
            await store.database.log.waitUntil(isSynced, context.signal);
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
        environment: context.environment,
    });

    // close the sockets that lapsed or stopped standing while the partition slept, as it opens
    running
        .sweep()
        .catch((error: unknown) =>
            TelemetryScope.of(
                import.meta.destack.package,
                context.environment.telemetry,
            ).captureException(error, { isEscaped: true }),
        );

    // serve the tunnel's sockets, which close as they lapse on the next wake or use
    return { packages: [store.package(relayPackage)], store, sockets: running };
}

/** Open a store following some of the universe's rows into the instance's memory. */
async function openReplica(
    context: WorkloadContext,
    server: ServerContext,
    follows: readonly Follow[],
    tunnels: (request: Request) => Promise<Response>,
    names: (request: Request) => Promise<Response>,
): Promise<ObjectStore> {
    // open the replica in a memory database, closed with the start
    const ephemeral = present(context.ephemeral, "the memory of the relay's instance");
    const database = await ephemeral.open(REPLICA);
    await database.migrate(REPLICA.tables);
    context.defer(() => database.close());

    // follow the rows through the home space's replicas, served at the relay installation
    return new ObjectStore({
        blobs: new MemoryBlobStorage(context.environment.crypto),
        servers: [
            implementPackage(relayPackage, { follows, mounts: { tunnels, names } }).at(server),
        ],
        chain: async () => undefined,
        principal: principal.installation.reference(server.scope, server.installation),
        publisher: () => server.remote,
        database,
        environment: context.environment,
        origin: { package: relayPackage },
    });
}

/** Refuse a routed name at a machine's tunnel, as the root alone routes names. */
function refuseNames(): Promise<Response> {
    throw RelayError.of("TUNNEL_CALL_NOT_FOUND", "name route at a machine's tunnel not found");
}

/** Call the relay installation itself at its own release, which the gateway routes calls by. */
function itselfOf(server: ServerContext): ReturnType<ServerContext["at"]> {
    const at = server.at(Egress.address(server.name, server.scope));

    return {
        ...at,
        fetch: (request) => {
            // state the release the call speaks, which picks the deployment serving it
            const headers = new Headers(request.headers);
            headers.set(VERSION_HEADER, import.meta.destack.package.version);

            return at.fetch(copyRequest(request, { headers }));
        },
    };
}

/** Answer one call of the root to a machine's tunnel. */
async function tunnelCall(request: Request, running: Tunnel): Promise<Response> {
    // require the root to address the partition's own machine
    const [, addressed, verb] = new URL(request.url).pathname.split("/");
    if (addressed !== running.machineId) {
        throw new TypeError(
            `tunnel of '${running.machineId}' got a call for '${addressed ?? "none"}'`,
        );
    }

    // accept a machine's WebSocket after its handshake for the same machine
    if (verb === "open") {
        const header = request.headers.get(HANDSHAKE_HEADER);
        if (header === null) {
            throw RelayError.of(
                "HANDSHAKE_REQUIRED",
                "handshake is required: the tunnel's opening has none",
            );
        }
        const handshake = Handshake.parse(JSON.parse(header));
        if (handshake.machineId !== running.machineId) {
            throw new TypeError(
                `tunnel of '${running.machineId}' got the handshake of '${handshake.machineId}'`,
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
        throw RelayError.of("TUNNEL_CALL_NOT_FOUND", `tunnel call '${verb ?? ""}' not found`);
    }
}

/** Restore the client's request a front door or the router handed on, with no header they added. */
function restore(request: Request): Request {
    const restored = ClientHead.restore(request);
    if (restored === undefined) {
        throw RelayError.of(
            "CLIENT_HEAD_REQUIRED",
            "client head is required: the handed request has none",
        );
    }

    return restored;
}
