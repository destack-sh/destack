import { serve } from "bun";
import { ResourceContext } from "@destack/resource/context";
import { ServiceMount } from "@destack/service";
import { BunHibernation } from "@destack/service/bun";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import {
    FORWARDED_URL_HEADER,
    HeldSocketDescription,
    Hibernation,
    ROUTE_PATH,
    SOCKET_HEADER,
    SOCKET_INSTANCE_HEADER,
} from "@destack/service/workload";
import type { Subject } from "@destack/sync";
import {
    implementRelay,
    RELAY_PACKAGE,
    type RelayImplementation,
    type RelayServiceOptions,
} from "../server/index.ts";

/** The Bun listener of a host's names and tunnels. */
type Listener = ReturnType<typeof serve>;

/** The one instance a test host runs the relay as. */
const INSTANCE = "instance-relay";

/** The origin a host reaches its installation's own server at, below the egress. */
const SELF_ORIGIN = "http://self.test";

/** The header a test host marks the installation's own calls with. */
const SELF_HEADER = "x-relay-self";

/** How long a server drains on close, in milliseconds. */
const DRAIN_MILLISECONDS = 1000;

/** The secret the host's held sockets deliver their events under. */
const SOCKET_SECRET = "secret";

/** The status of a handled socket event (RFC 9110 15.3.5). */
const NO_CONTENT = 204;

/** The status of a path outside the installation's mount (RFC 9110 15.5.5). */
const NOT_FOUND = 404;

/** The status of an upgrade the host could not hold (RFC 9110 15.5.22). */
const UPGRADE_REQUIRED = 426;

/** A relay installation served in this process, as a machine hosts it. */
export class RelayHost {
    /** The relay's service, its router and tunnels. */
    readonly service: RelayImplementation;
    /** The server of the relay's service. */
    readonly server: Server;
    /** The listener of names and tunnels. */
    readonly #listener: Listener;

    /** Keep a started relay with its server and listener. */
    private constructor(service: RelayImplementation, server: Server, listener: Listener) {
        this.service = service;
        this.server = server;
        this.#listener = listener;
    }

    /** Serve a relay installation on a port, as the machine hosting it would. */
    static start(
        port: number,
        installation: Subject,
        options: Omit<RelayServiceOptions, "installation" | "hibernation" | "origin">,
    ): RelayHost {
        // keep the tunnels' sockets open, delivering their events to the relay's tunnels
        let service: RelayImplementation | undefined;
        const sockets = new BunHibernation(async (_instanceId, _path, request) => {
            // describe the socket
            const tunnels = RelayHost.#require(service).tunnels;
            const description = HeldSocketDescription.parse(
                JSON.parse(request.headers[SOCKET_HEADER] ?? "null"),
            );
            const socket = held.socket(description);

            // hand over a close
            if (request.method === "DELETE") {
                await tunnels.close(socket);
            }
            // hand over a message
            else {
                const body = new Uint8Array(await new Response(request.body).arrayBuffer());
                await tunnels.message(socket, body);
            }

            return new Response(null, { status: NO_CONTENT });
        });
        const held = new Hibernation(
            {
                url: "http://host.test/.destack/socket",
                fetch: (request) => sockets.answer(INSTANCE, request),
            },
            SOCKET_SECRET,
            (error) => options.report(error),
        );

        // reach the installation's own server below its mount, as the installation
        let server: Server | undefined;
        const itself = {
            url: SELF_ORIGIN,
            fetch: async (request: Request) => {
                // strip the mount, refusing a path outside it
                const routed = ServiceMount.of(request);
                if (routed === undefined) {
                    return new Response(null, { status: NOT_FOUND });
                }

                // call as the installation
                routed.request.headers.set(SELF_HEADER, "1");

                return RelayHost.#require(server).fetch(routed.request);
            },
        };

        // implement the relay at the installation's root, holding every partition
        service = implementRelay({
            ...options,
            origin: `http://127.0.0.1:${port}`,
            installation: {
                id: installation.id,
                scope: installation.scope,
                at: () => itself,
                publisherAt: () => {
                    throw new TypeError("the test host serves every partition at the root");
                },
            },
            hibernation: held,
        });

        // serve the relay's service, its own calls as the installation
        const now = Date.now();
        const self = {
            subject: installation,
            subjects: [installation],
            credential: {
                kind: "installation",
                id: INSTANCE,
                audience: RELAY_PACKAGE.id,
                scope: installation.scope,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            },
        };
        server = Server.start({
            ...service,
            clock: options.clock,
            audience: RELAY_PACKAGE.id,
            resources: new ResourceContext(),
            health: new Health(service.service.name),
            authenticate: async (request) => (request.headers.has(SELF_HEADER) ? self : undefined),
            authorizeMachine: async () => {},
            drainTimeout: DRAIN_MILLISECONDS,
        });

        // hand names to the relay, keeping open the sockets its tunnels accept
        const started = server;
        const listener = serve({
            hostname: "127.0.0.1",
            port,
            fetch: async (request, bun) => {
                // hand the request to the relay's route path, as its route would, under the URL it asked for
                const url = new URL(request.url);
                const host = request.headers.get("host") ?? url.host;
                const headers = new Headers(request.headers);
                headers.set(FORWARDED_URL_HEADER, `http://${host}${url.pathname}${url.search}`);
                const answer = await started.fetch(
                    new Request(`http://relay.test${ROUTE_PATH}`, {
                        method: request.method,
                        headers,
                        body: request.body,
                    }),
                );

                // hold a socket the relay accepted for an upgrade
                if (
                    request.headers.get("upgrade") !== "websocket" ||
                    !answer.headers.has(SOCKET_HEADER)
                ) {
                    return answer;
                }
                const accepted = new Headers(answer.headers);
                accepted.set(SOCKET_INSTANCE_HEADER, INSTANCE);

                return sockets.upgrade(bun, request, new Response(null, { headers: accepted }))
                    ? undefined
                    : new Response(null, { status: UPGRADE_REQUIRED });
            },
            websocket: sockets.websocket,
        });

        return new RelayHost(service, started, listener);
    }

    /** The relay's URL machines open their tunnels at. */
    get url(): string {
        return this.service.relay.url;
    }

    /** The port the listener bound. */
    get port(): number {
        const port = this.#listener.port;
        if (port === undefined) {
            throw new TypeError("the relay listens on no port");
        }

        return port;
    }

    /** Stop listening, ending every tunnel, then the relay's server. */
    async close(): Promise<void> {
        await this.#listener.stop(true);
        await this.server.close();
    }

    /** Read a value the host keeps once it started. */
    static #require<Value>(value: Value | undefined): Value {
        if (value === undefined) {
            throw new TypeError("the relay host has not started");
        }

        return value;
    }
}
