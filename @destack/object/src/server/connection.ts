import { CALLER_HEADER } from "@destack/access";
import { ServerPeer } from "@orpc/standard-server-peer";
import type { StandardRequest, StandardResponse } from "@orpc/standard-server";
import { reportError } from "../error/report.ts";
import { BOOKMARK_HEADER } from "../procedure/bookmark.ts";
import { present, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import type { ClientSocket, Environment, WebSocketUpgrade } from "@destack/environment";
import {
    type Hibernation,
    HibernationHost,
    type HostListening,
    type HostSocketAttachment,
    type Socket,
} from "../workload/socket.ts";
import type { Server } from "./http.ts";
import { RequestContext, type RequestContextOptions } from "./context.ts";
import type { StandardRPCHandler } from "@orpc/server/standard";

/** The value a host keeps beside a connection's socket: the package whose server accepted it. */
export const ObjectConnectionAttachment = schema.object({
    /** The package. */
    connection: PackageId,
});
/** The value a host keeps beside a connection's socket. */
export type ObjectConnectionAttachment = schema.Infer<typeof ObjectConnectionAttachment>;

/** The upgrade's credentials, which no call takes over: each call carries its own. */
const UPGRADE_CREDENTIALS = ["authorization", "sec-websocket-protocol", CALLER_HEADER] as const;

/** The answer to a call frame no procedure matches. */
const NOT_FOUND: StandardResponse = {
    status: 404,
    headers: {},
    body: "procedure not found",
};

/** The origin a connection host addresses its sockets at, never leaving the process. */
const CONNECTIONS_ORIGIN = "http://connections.local";

/** The instance a process's connection host keeps every socket under. */
const CONNECTIONS_INSTANCE = "connections";

/** One client's socket at a server: its calls and follows, as the upgrade's caller. */
export class ObjectConnectionServer {
    /** The socket the host keeps for the server. */
    readonly socket: Socket;
    /** The upgrade request, whose URL and headers every call reads beside its own. */
    readonly #upgrade: Request;
    /** Verify a call's caller as the server verifies a request's. */
    readonly #authenticate: (request: Request) => Promise<RequestContextOptions>;
    /** Read the procedures the calls run, the server's current ones. */
    readonly #procedures: () => StandardRPCHandler<RequestContext>;
    /** The peer decoding the calls and encoding their answers and pages. */
    readonly #peer: ServerPeer;
    /** The telemetry a failed call no one awaits reports to. */
    readonly #telemetry: Environment["telemetry"];
    /** The calls running now, each settling once its answer or its follow's last page went out. */
    readonly #running = new Set<Promise<void>>();

    /** Serve a socket's calls as the caller an upgrade verified. */
    constructor(
        socket: Socket,
        upgrade: Request,
        procedures: () => StandardRPCHandler<RequestContext>,
        telemetry: Environment["telemetry"],
        authenticate: (request: Request) => Promise<RequestContextOptions>,
    ) {
        // take the socket, the upgrade's verified caller and the procedures, decoding calls as a peer
        this.socket = socket;
        this.#upgrade = upgrade;
        this.#procedures = procedures;
        this.#telemetry = telemetry;
        this.#authenticate = authenticate;
        this.#peer = new ServerPeer((message) =>
            socket.send(typeof message === "string" ? message : new TextDecoder().decode(message)),
        );
    }

    /** Take one text frame, starting its call without waiting for the answer it sends later. */
    message(frame: string): void {
        const handled = this.#peer
            .message(frame, (request) => this.#answer(request))
            .then(() => undefined)
            .catch((error: unknown) => {
                reportError(error, this.#telemetry);
            });
        this.#track(handled);
    }

    /** End every call and follow of a closed socket, settling once each one finished. */
    async close(): Promise<void> {
        this.#peer.close();
        while (this.#running.size > 0) {
            await Promise.allSettled(this.#running);
        }
    }

    /** Count a call as running until it settles. */
    #track(running: Promise<void>): void {
        this.#running.add(running);
        void running.finally(() => this.#running.delete(running));
    }

    /** Answer one call in its own context: the upgrade's caller, the call's headers and abort. */
    async #answer(request: StandardRequest): Promise<StandardResponse> {
        // read the call as a request beside the upgrade, without the upgrade's credentials, aborted as its client stops it
        const headers = new Headers(this.#upgrade.headers);
        for (const name of UPGRADE_CREDENTIALS) {
            headers.delete(name);
        }
        for (const [name, value] of Object.entries(request.headers)) {
            // join a repeated header, leaving out an absent one
            if (Array.isArray(value)) {
                headers.set(name, value.join(", "));
            } else if (value !== undefined) {
                headers.set(name, value);
            }
        }
        const { signal } = request;
        const called = new Request(this.#upgrade.url, {
            headers,
            ...(signal === undefined ? {} : { signal }),
        });

        // run the procedure in a context of its own, its caller verified for this call
        const context = new RequestContext(called, await this.#authenticate(called));
        const { response } = await this.#procedures().handle(
            { ...request, body: () => Promise.resolve(request.body) },
            { context },
        );
        if (response === undefined) {
            return NOT_FOUND;
        }

        // answer the watermarks the call observed, for the client's later calls to wait for
        const { observed } = context;
        const answered =
            observed.watermarks.length === 0
                ? response.headers
                : { ...response.headers, [BOOKMARK_HEADER]: observed.format() };

        return { ...response, headers: answered };
    }
}

/** The connections of a process's servers, their sockets kept in the process. */
export class ObjectConnectionHost {
    /** The servers' side of the kept sockets, which each served server takes. */
    readonly hibernation: Hibernation;
    /** The host keeping the sockets. */
    readonly #host: HibernationHost;
    /** The servers taking the connections' frames, by package. */
    readonly #servers = new Map<PackageId, Pick<Server, "sockets">>();

    /** Keep sockets in this process, capturing failed deliveries on a telemetry. */
    constructor(telemetry: Environment["telemetry"]) {
        // deliver each connection's events to the server of its package
        this.#host = new HibernationHost(async (_instance, path, request) => {
            const delivered = new Request(`${CONNECTIONS_ORIGIN}${path}`, request);
            await this.hibernation.receive(delivered, {
                message: (socket, message) =>
                    this.#server(socket.attachment).message(socket, message),
                close: (socket) => this.#server(socket.attachment).close(socket),
            });

            return new Response(null, { status: 204 });
        }, telemetry);

        // send through the host directly as its one instance in this process
        this.hibernation = this.#host.instance(CONNECTIONS_INSTANCE);
    }

    /** Report whether a kept socket is a connection host's, by the instance that accepted it. */
    static isKept(attachment: HostSocketAttachment): boolean {
        return attachment.instance === CONNECTIONS_INSTANCE;
    }

    /** The host keeping the sockets, which a listener or a Durable Object hands their events. */
    get host(): HibernationHost {
        return this.#host;
    }

    /** Answer a listener's requests through a fetch, keeping the sockets its answers accept. */
    listen(fetch: (request: Request) => Promise<Response>): HostListening {
        return this.#host.listen(fetch);
    }

    /** Take a package's connections at a server, replacing its earlier one. */
    serve(packageId: PackageId, server: Pick<Server, "sockets">): void {
        this.#servers.set(packageId, server);
    }

    /** Open a socket through a fetch answering its upgrade, keeping the accepted socket here. */
    open(fetch: (upgrade: Request) => Promise<Response>, upgrade: WebSocketUpgrade): ClientSocket {
        return this.#host.open(fetch, upgrade);
    }

    /** Read the server taking a connection's frames by the package it names. */
    #server(attachment: unknown): Pick<Server, "sockets">["sockets"] {
        const { connection } = ObjectConnectionAttachment.parse(attachment);
        const served = this.#servers.get(PackageId.parse(connection));

        return present(served, `the server of package '${connection}'`).sockets;
    }
}
