import relayPackage from "../package.ts";
import { connection } from "../object/index.ts";
import { Caller } from "@destack/access";
import { telemetry } from "@destack/telemetry";
import type { Environment, Listener } from "@destack/environment";
import { ResourceContext } from "@destack/resource/context";
import { PackageMount, reportError } from "@destack/object";
import { Health, Server } from "@destack/object/server";
import {
    HibernationHost,
    FORWARDED_URL_HEADER,
    PARTITION_HEADER,
    SocketDescription,
    Hibernation,
    ROUTE_PATH,
    SOCKET_HEADER,
    SOCKET_INSTANCE_HEADER,
} from "@destack/object/workload";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import type { DatabaseConnection, Snapshot } from "@destack/db";
import { Scope, type Subject, Uplink } from "@destack/sync";
import { createClient } from "@destack/object/client";
import { type Identifier, schema } from "@destack/schema";
import {
    implementRelay,
    type RelayImplementation,
    type RelayPackageOptions,
    Tunnel,
} from "../server/index.ts";

/** The attachment of a hibernated tunnel socket: the machine it connects. */
const SocketAttachment = schema.object({
    /** The machine the socket connects. */
    machineId: schema.identifier("machine"),
});

/** The instruments capturing the failures of a test host's sockets. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The one instance a test host runs the relay as. */
const INSTANCE = "instance-relay";

/** The origin of a host's installation's own server, below the egress. */
const SELF_ORIGIN = "http://self.test";

/** The header a test host marks the installation's own calls with. */
const SELF_HEADER = "x-relay-self";

/** How long a server drains on close, in milliseconds. */
const DRAIN_MILLISECONDS = 1000;

/** The secret the host's hibernated sockets deliver their events under. */
const SOCKET_SECRET = "secret";

/** The status of a handled socket event (RFC 9110 15.3.5). */
const NO_CONTENT = 204;

/** The status of a path outside the installation's mount (RFC 9110 15.5.5). */
const NOT_FOUND = 404;

/** The status of an upgrade the host failed to complete (RFC 9110 15.5.22). */
const UPGRADE_REQUIRED = 426;

/** The relay package served at the root or in a partition, with its server. */
interface Served<Implementation extends RelayImplementation = RelayImplementation> {
    /** The package's implementation. */
    readonly implementation: Implementation;
    /** Its server. */
    readonly server: Server;
}

/** The relay package's root implementation, its router of names. */
type RootImplementation = Extract<RelayImplementation, { readonly kind: "root" }>;

/** A relay installation served in this process with its root and partitions. */
export class RelayHost {
    /** The relay package's root implementation, its router of names. */
    readonly implementation: RootImplementation;
    /** The root's server. */
    readonly server: Server;
    /** The listener of names and tunnels. */
    readonly #listener: Pick<Listener<unknown>, "url" | "stop">;
    /** Serve a partition, starting it on its first use. */
    readonly #partition: (partition: string) => Promise<Served>;
    /** The partitions served so far. */
    readonly #partitions: ReadonlyMap<string, Promise<Served>>;

    /** Create a host of a started relay with its root, its partitions and its listener. */
    private constructor(
        root: Served<RootImplementation>,
        partitions: {
            readonly serve: (partition: string) => Promise<Served>;
            readonly served: ReadonlyMap<string, Promise<Served>>;
        },
        listener: Pick<Listener<unknown>, "url" | "stop">,
    ) {
        // serve the root, the partitions and the listener
        this.implementation = root.implementation;
        this.server = root.server;
        this.#partition = partitions.serve;
        this.#partitions = partitions.served;
        this.#listener = listener;
    }

    /** List the connections the partitions served so far record, in start order. */
    async connections(): Promise<string[]> {
        const ids: string[] = [];
        for (const served of this.#partitions.values()) {
            const rows = await (
                await served
            ).implementation.objects.database
                .select({ id: connection.table.id })
                .from(connection.table);
            ids.push(...rows.map((row) => row.id));
        }

        return ids;
    }

    /** Read a machine's tunnel in its partition, starting the partition on its first use. */
    async tunnel(machineId: Identifier<"machine">): Promise<Tunnel> {
        const { implementation } = await this.#partition(Tunnel.id(machineId));
        if (implementation.kind !== "partition") {
            throw new TypeError(`partition of machine ${machineId} serves no tunnel`);
        }

        return implementation.tunnel;
    }

    /** Insert a relay's home space row, as its store stores it where the space is placed. */
    static async home(
        database: DatabaseConnection,
        universe: Snapshot,
        installation: Subject,
    ): Promise<void> {
        const [home] = await Scope.read(universe, [installation.scope]);
        if (home === undefined) {
            throw new TypeError(`the relay's home space ${installation.scope} has no scope row`);
        }
        await database.insert(Scope.table).values(home);
    }

    /** Serve a relay installation on a port, a free one for zero, as its machine would. */
    static start(
        installation: Subject,
        options: Omit<
            RelayPackageOptions,
            "server" | "hibernation" | "origin" | "fetch" | "clock"
        > & {
            readonly environment: Environment;
            readonly port: number;
            /** Open a partition's own database and events. */
            readonly partitions: (
                partition: string,
            ) => Promise<Pick<RelayPackageOptions, "database" | "events">>;
            /** Report a failure of the root's or a partition's server, beside its telemetry. */
            readonly report?: (error: unknown) => void;
        },
    ): RelayHost {
        // the root once served, and the host once started
        let root: Served<RootImplementation> | undefined;
        let host: RelayHost | undefined;

        // accept the tunnels' sockets, delivering their events to their machines' partitions
        const sockets = new HibernationHost(async (_instanceId, _path, request) => {
            // describe the socket, and find its machine's tunnel
            const description = SocketDescription.parse(
                JSON.parse(request.headers[SOCKET_HEADER] ?? "null"),
            );
            const socket = hibernation.socket(description);
            const { machineId } = SocketAttachment.parse(description.attachment);
            const running = await RelayHost.#require(host).tunnel(machineId);

            // hand over a close or a message, answering a refusal's status as the instance's server does
            try {
                // hand over a close
                if (request.method === "DELETE") {
                    await running.close(socket);
                }
                // hand over a message
                else {
                    const body = new Uint8Array(await new Response(request.body).arrayBuffer());
                    await running.message(socket, body);
                }
            } catch (error) {
                return new Response(null, { status: reportError(error).status });
            }

            return new Response(null, { status: NO_CONTENT });
        });
        const hibernation = new Hibernation(
            {
                url: "http://host.test/.destack/socket",
                fetch: (request) => sockets.answer(INSTANCE, request),
            },
            SOCKET_SECRET,
            (error) => instruments.captureException(error),
        );

        // call the installation's own server below its mount, as the installation
        const itself = {
            url: SELF_ORIGIN,
            fetch: async (request: Request) => {
                // strip the mount, refusing a path outside it
                const routed = PackageMount.of(request);
                if (routed === undefined) {
                    return new Response(null, { status: NOT_FOUND });
                }

                // call as the installation, in the partition the call names or at the root
                routed.request.headers.set(SELF_HEADER, "1");
                const partition = routed.request.headers.get(PARTITION_HEADER);
                const served =
                    partition === null ? RelayHost.#require(root) : await serve(partition);

                return served.server.fetch(routed.request);
            },
        };

        // hand names to the relay, leaving open the sockets its tunnels accept
        const { environment, port, partitions: openPartition, report, ...relay } = options;
        const { clock, network } = environment;
        const listener = network.serve({
            hostname: "127.0.0.1",
            port,
            fetch: async (request, served) => {
                // hand the request to the relay's route path, as its route would, under the URL it asked for
                const url = new URL(request.url);
                const authority = request.headers.get("host") ?? url.host;
                const headers = new Headers(request.headers);
                headers.set(
                    FORWARDED_URL_HEADER,
                    `http://${authority}${url.pathname}${url.search}`,
                );
                const answer = await RelayHost.#require(root).server.fetch(
                    new Request(`http://relay.test${ROUTE_PATH}`, {
                        method: request.method,
                        headers,
                        body: request.body,
                    }),
                );

                // record a socket the relay accepted for an upgrade
                if (
                    request.headers.get("upgrade") !== "websocket" ||
                    !answer.headers.has(SOCKET_HEADER)
                ) {
                    return answer;
                }
                const accepted = new Headers(answer.headers);
                accepted.set(SOCKET_INSTANCE_HEADER, INSTANCE);

                return sockets.upgrade(served, request, new Response(null, { headers: accepted }))
                    ? undefined
                    : new Response(null, { status: UPGRADE_REQUIRED });
            },
            websocket: sockets.websocket,
        });

        // serve the relay package at the root or in a partition, verifying its own calls as the installation
        const self = (now: number) => ({
            caller: Caller.of(installation),
            credential: {
                kind: "installation",
                id: INSTANCE,
                audience: relayPackage.id,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            },
        });
        const start = (
            partition: string | undefined,
            storage: Pick<RelayPackageOptions, "database" | "events">,
        ): Served => {
            const implementation = implementRelay({
                ...relay,
                ...storage,
                fetch: (request) => network.fetch(request),
                clock,
                origin: listener.url.origin,
                server: {
                    installation: installation.id,
                    name: "relay",
                    space: installation.scope,
                    ...(partition === undefined ? {} : { partition }),
                    at: () => itself,
                    // reach the root's store from a partition, as a machine's partitions reach their root
                    publisherAt: () =>
                        Uplink.of(
                            () =>
                                createClient(relayPackage, {
                                    url: `${SELF_ORIGIN}${PackageMount.path(relayPackage.id)}`,
                                    fetch: (request) => itself.fetch(request),
                                }).store,
                        ),
                },
                hibernation,
            });
            const server = Server.start({
                ...implementation,
                clock,
                resources: new ResourceContext(),
                health: new Health(implementation.package.name),
                authenticate: async (request) =>
                    request.headers.has(SELF_HEADER) ? self(clock.now()) : undefined,
                authorizeMachine: async () => {},
                drainTimeout: DRAIN_MILLISECONDS,
                ...(report === undefined ? {} : { report }),
            });

            return { implementation, server };
        };

        // serve the root at once, and each partition on its first use, as a machine's partitions start
        const started = start(undefined, relay);
        if (started.implementation.kind !== "root") {
            throw new TypeError("the relay's root serves no router");
        }
        root = { implementation: started.implementation, server: started.server };
        const partitions = new Map<string, Promise<Served>>();
        const serve = (partition: string): Promise<Served> => {
            // serve a started partition, or start it
            const existing = partitions.get(partition);
            if (existing !== undefined) {
                return existing;
            }
            const served = openPartition(partition).then((storage) => start(partition, storage));
            partitions.set(partition, served);

            return served;
        };
        host = new RelayHost(root, { serve, served: partitions }, listener);

        return host;
    }

    /** The relay's URL machines open their tunnels at. */
    get url(): string {
        return this.implementation.relay.url;
    }

    /** The port the listener bound. */
    get port(): number {
        return Number(this.#listener.url.port);
    }

    /** Stop listening, ending every tunnel, then the relay's servers. */
    async close(): Promise<void> {
        await this.#listener.stop(true);
        await Promise.all(
            [...this.#partitions.values()].map(async (served) => (await served).server.close()),
        );
        await this.server.close();
    }

    /** Read a value the host has once it started. */
    static #require<Value>(value: Value | undefined): Value {
        if (value === undefined) {
            throw new TypeError("the relay host has not started");
        }

        return value;
    }
}
