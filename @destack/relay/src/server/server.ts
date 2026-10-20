import type { ClientOf } from "@destack/runtime/client";
import {
    FORWARDED_URL_HEADER,
    type Hibernation,
    type ServerContext,
    PARTITION_HEADER,
    ROUTE_PATH,
} from "@destack/runtime/workload";
import type accountPackage from "@destack/account/package";
import { DOMAINS } from "@destack/account/address";
import { account, directoryShape, identityShape } from "@destack/account/object";
import { principal } from "@destack/access";
import { Scope, Uplink } from "@destack/sync";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { EventStore } from "@destack/event";
import { defineServer, ObjectStore } from "@destack/object/server";
import { present, schema } from "@destack/schema";
import type { CallKey } from "@destack/runtime/request";
import { Egress } from "@destack/runtime";
import { CallError } from "@destack/runtime/error";
import type { PackageImplementation } from "@destack/runtime/server";
import { connection, tunnel } from "../object/index.ts";
import relayPackage from "../package.ts";
import { NameController } from "./name-controller.ts";
import { Relay, type RelayOptions } from "./relay.ts";
import { HANDSHAKE_HEADER, Handshake, Tunnel, TUNNEL_PATH } from "./tunnel.ts";
import { LapseController } from "./lapse-controller.ts";
import { Follow } from "@destack/object";
import { Clock } from "@destack/environment";

/** What the relay package runs as. */
export interface RelayPackageOptions extends Omit<RelayOptions, "store" | "itself"> {
    /** The relay's database, which its replicas follow into. */
    readonly database: DatabaseConnection;
    /** The event log recording the relay's calls, tunnel openings and closes among them. */
    readonly events: EventStore;
    /** The key the relay's journal fingerprints sensitive values under. */
    readonly callKey: CallKey;
    /** The universe's account package, called as the installation. */
    readonly accounts: ClientOf<typeof accountPackage>;
    /** The relay installation's server context: its space, partition, and calls to itself. */
    readonly server: Pick<
        ServerContext,
        "installation" | "name" | "space" | "partition" | "at" | "publisherAt"
    >;
    /** The host of the instance's hibernating WebSockets, its tunnels' connections. */
    readonly hibernation: Hibernation;
    /** The clock the store and its tunnels run at. */
    readonly clock: Clock;
}

/** The relay package as one store serves it: the root's router, or one machine's tunnel. */
export type RelayImplementation = PackageImplementation & {
    /** The relay installation's store at the root or in the partition. */
    readonly objects: ObjectStore;
} & (
        | {
              /** The root, routing names. */
              readonly kind: "root";
              /** The router of names to its machines' tunnels. */
              readonly relay: Relay;
          }
        | {
              /** A machine's partition, running its tunnel. */
              readonly kind: "partition";
              /** The partition's tunnel. */
              readonly tunnel: Tunnel;
          }
    );

/** Implement the relay package at the root, routing names, or in a machine's tunnel partition. */
export function implementRelay(options: RelayPackageOptions): RelayImplementation {
    // route names at the root, and run a machine's tunnel in its partition
    const { server } = options;
    const space = server.space;
    const isRoot = server.partition === undefined || server.partition === space;

    return isRoot
        ? implementRoot(options)
        : implementPartition(options, present(server.partition, "the tunnel's partition"));
}

/** Implement the root: the router of names, which reaches each tunnel in its partition. */
function implementRoot(options: RelayPackageOptions): RelayImplementation {
    // implement the tunnel rows, whose renames and closes reach the machine's partition
    const { server } = options;
    const space = server.space;
    const tunnels = tunnel.implement({
        methods: {
            rename: async (call, next) => {
                // rename, and tell the machine's tunnel in its partition
                const renamed = await next();
                await relay.tunnel(Tunnel.machine(call.target.id), "rename", {
                    method: "POST",
                    body: JSON.stringify({ name: renamed.name }),
                });

                return renamed;
            },
            close: async (call) => {
                // end the tunnel's connections in its partition
                const ended = await relay.tunnel(Tunnel.machine(call.target.id), "close", {
                    method: "POST",
                });
                const closed = schema
                    .object({ closed: schema.boolean() })
                    .parse(await ended.json());

                // remove a revoked machine's tunnel, and leave a keyless one's for its next key
                if (call.input.reason === "machine-revoked") {
                    await call.remove();
                }

                return closed;
            },
        },
    });
    const objects: ObjectStore = new ObjectStore({
        servers: [
            defineServer(relayPackage, {
                objects: [tunnels, connection],
                // refuse the tunnels' calls at the root, which runs no tunnel
                adapters: {
                    tunnels: async () => {
                        throw new CallError("NOT_FOUND", {
                            message: "the relay's root runs no tunnel",
                        });
                    },
                },
                follows: [
                    Follow.of(directoryShape, { keys: [{ key: { scope: Scope.universe.id } }] }),
                    Follow.of(identityShape, { keys: [{ key: { scope: Scope.universe.id } }] }),
                ],
            }),
        ],
        // root the chain above the home space where its row lives
        chain: async (view) => {
            const [row] = await Scope.read(view, [space]);

            return row === undefined ? undefined : { kind: "home", parent: row.parent };
        },
        principal: principal.installation.reference(space, server.installation),
        policies: [account],
        database: options.database,
        callKey: options.callKey,
        events: options.events,
        clock: options.clock,
        origin: { package: relayPackage },
        uplink: Uplink.of(() => options.accounts.store),
    });

    // call the tunnels in their partitions through the installation itself
    const relay = new Relay({
        ...options,
        store: objects,
        itself: server.at(Egress.address(server.name, space)),
    });

    // serve the package, handing a route's requests to the router, running the names' controller
    const implementation = objects.package(relayPackage);

    return {
        ...implementation,
        controllers: [...(implementation.controllers ?? []), new NameController(relay)],
        fetch: async (request, context) =>
            new URL(request.url).pathname === ROUTE_PATH
                ? relay.fetch(restore(request))
                : implementation.fetch?.(request, context),
        objects,
        kind: "root",
        relay,
    };
}

/** Implement a machine's partition: its tunnel, its connection rows and their lapses. */
function implementPartition(options: RelayPackageOptions, partition: string): RelayImplementation {
    // serve the tunnel's connection rows in its partition, answering the root's calls alone
    const { server } = options;
    const space = server.space;
    const self = principal.installation.reference(space, server.installation);
    const universe = Uplink.of(() => options.accounts.store);
    const objects: ObjectStore = new ObjectStore({
        servers: [
            defineServer(relayPackage, {
                objects: [connection],
                // follow the universe's identity, verifying renewals by its keys
                follows: [
                    Follow.of(identityShape, { keys: [{ key: { scope: Scope.universe.id } }] }),
                ],
                adapters: {
                    tunnels: (request) => tunnelCall(request, running),
                },
            }),
        ],
        // follow the tunnel's chain from the installation's root, which opens the tunnel and relates its server
        chain: async () => ({ kind: "follower", parent: space }),
        principal: self,
        // decide the connections under the tunnel's access, which the root stores and the partition replicates
        policies: [account, tunnel],
        database: options.database,
        callKey: options.callKey,
        events: options.events,
        clock: options.clock,
        origin: { package: relayPackage },
        partition,
        // reach the universe's rows at the account package, and the tunnel's chain at the root
        publisher: (home) =>
            home === Scope.universe.id
                ? universe
                : server.publisherAt(Egress.address(server.name, space), relayPackage.id),
    });

    // run the machine's tunnel over the partition's sockets
    const running = new Tunnel(Tunnel.machine(partition), {
        store: objects,
        hibernation: options.hibernation,
        url: new URL(TUNNEL_PATH, options.origin).href,
        tokens: Tunnel.verifier(options.issuer, Snapshot.live(options.database)),
        domains: options.domains ?? DOMAINS,
    });

    // serve the package, running the lapses of the tunnel's connections
    const implementation = objects.package(relayPackage);

    return {
        ...implementation,
        controllers: [...(implementation.controllers ?? []), new LapseController(running)],
        objects,
        kind: "partition",
        tunnel: running,
    };
}

/** Answer one call of the root to a machine's tunnel. */
async function tunnelCall(request: Request, running: Tunnel): Promise<Response> {
    // accept a machine's WebSocket after its handshake
    const [, , verb] = new URL(request.url).pathname.split("/");
    if (verb === "open") {
        const handshake = Handshake.parse(
            JSON.parse(request.headers.get(HANDSHAKE_HEADER) ?? "null"),
        );

        return running.accept(handshake);
    }
    // forward a request through the tunnel under the host it asked for
    else if (verb === "forward") {
        return running.fetch(restore(request));
    }
    // tell the machine its new name
    else if (verb === "rename") {
        const { name } = schema
            .object({ name: schema.string().min(1) })
            .parse(await request.json());
        await running.rename(name);

        return new Response(null, { status: 204 });
    }
    // end the tunnel's connections and forget their rows
    else if (verb === "close") {
        return Response.json({ closed: await running.end() });
    }
    // refuse any other call
    else {
        throw new CallError("NOT_FOUND", { message: `no tunnel call ${verb ?? ""}` });
    }
}

/** Restore a request a route or the router handed the relay to the URL it asked for. */
function restore(request: Request): Request {
    // read the URL it asked for
    const url = request.headers.get(FORWARDED_URL_HEADER);
    if (url === null) {
        throw new CallError("BAD_REQUEST", { message: "the handed request names no URL" });
    }

    // ask for it as the client did, without the handing headers
    const restored = new Request(url, request);
    restored.headers.delete(FORWARDED_URL_HEADER);
    restored.headers.delete(PARTITION_HEADER);

    return restored;
}
