import { schema, type JsonValue } from "@destack/schema";
import type { Listener, ServerSocket, ServerSocketHandler } from "@destack/environment";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { CallError } from "../error/index.ts";
import type { Fetch } from "../runtime/fetch.ts";

/** The path where a host and an instance exchange a hibernated socket's messages and closes. */
export const SOCKET_PATH = "/.destack/socket";

/** The header with a hibernated socket beside its event. */
export const SOCKET_HEADER = "Destack-Socket";

/** The header that identifies the instance that accepted an upgrade. */
export const SOCKET_INSTANCE_HEADER = "Destack-Socket-Instance";

/** The most bytes in one WebSocket message: 1 MiB. */
const MESSAGE_BYTES = 1024 * 1024;

/** The instruments capturing the closes instances refuse. */
const instruments = telemetry.scope(import.meta.destack.package);

/** A hibernated socket as a host describes it: its identifier and the value stored beside it. */
export const SocketDescription = schema.object({
    /** The socket's identifier, unique among the instance's hibernated sockets. */
    id: schema.string().min(1),
    /** The value the instance stored beside the socket when it accepted it. */
    attachment: schema.json(),
});
/** A hibernated socket as a host describes it. */
export type SocketDescription = schema.Infer<typeof SocketDescription>;

/** An instance's acceptance of a WebSocket upgrade. */
export const SocketAcceptance = schema.object({
    /** The socket's identifier, the instance's pick, unique among its hibernated sockets. */
    id: schema.string().min(1),
    /** The value stored beside the socket. */
    attachment: schema.json(),
    /** The subprotocol the upgrade answers with, among those the client offered. */
    protocol: schema.string().min(1).exactOptional(),
    /** A text message and the answer the host sends to it, such as a liveness ping. */
    respond: schema
        .object({
            /** The message. */
            request: schema.string().min(1),
            /** The answer. */
            response: schema.string().min(1),
        })
        .exactOptional(),
});
/** An instance's acceptance of a WebSocket upgrade. */
export type SocketAcceptance = schema.Infer<typeof SocketAcceptance>;

/** One WebSocket a host hibernates for an instance while it sleeps. */
export interface Socket {
    /** The socket's identifier. */
    readonly id: string;
    /** The value the instance stored beside the socket. */
    readonly attachment: JsonValue;
    /** Send one binary message, in order with the socket's other sends. */
    readonly send: (message: Uint8Array<ArrayBuffer>) => void;
    /** Close the socket. */
    readonly close: () => void;
}

/** What a workload does with its hibernated sockets' messages and closes. */
export interface SocketHandler {
    /** Take one binary message a hibernated socket received. */
    readonly message: (socket: Socket, message: Uint8Array<ArrayBuffer>) => Promise<void>;
    /** Take the close of a hibernated socket. */
    readonly close: (socket: Socket) => Promise<void>;
}

/** Where an instance connects to the sockets its host hibernates. */
export interface SocketHost {
    /** The URL of the hibernated sockets. */
    readonly url: string;
    /** Send a request to the host. */
    readonly fetch: Fetch;
}

/** The WebSockets a host hibernates for an instance while it sleeps. */
export class Hibernation {
    /** Where the host hibernates the sockets. */
    readonly #host: SocketHost;
    /** The secret the host and the instance prove each other's requests with. */
    readonly #secret: string;
    /** Report a failed send or close that no caller awaits. */
    readonly #report: (error: unknown) => void;
    /** The last send or close of each socket, which the next one follows. */
    readonly #pending = new Map<string, Promise<void>>();

    /** Connect to the sockets at a host with an instance's secret. */
    constructor(host: SocketHost, secret: string, report: (error: unknown) => void) {
        this.#host = host;
        this.#secret = secret;
        this.#report = report;
    }

    /** Answer a WebSocket upgrade by accepting it. */
    accept(acceptance: SocketAcceptance): Response {
        return new Response(null, {
            headers: { [SOCKET_HEADER]: JSON.stringify(SocketAcceptance.parse(acceptance)) },
        });
    }

    /** List the hibernated sockets with their stored values. */
    async list(): Promise<readonly Socket[]> {
        // read the descriptions the host stores
        const response = await this.#host.fetch(
            new Request(this.#host.url, { headers: this.#headers() }),
        );
        if (!response.ok) {
            throw new CallError("BAD_GATEWAY", {
                message: `the host cannot list hibernated sockets: ${response.status}`,
            });
        }

        return schema
            .array(SocketDescription)
            .parse(await response.json())
            .map((description) => this.socket(description));
    }

    /** Address a hibernated socket the host describes. */
    socket(description: SocketDescription): Socket {
        const url = `${this.#host.url}/${encodeURIComponent(description.id)}`;

        return {
            ...description,
            send: (message) =>
                this.#enqueue(description.id, { method: "POST", body: message }, url),
            close: () => this.#enqueue(description.id, { method: "DELETE" }, url),
        };
    }

    /** Wait until a socket's sends and closes so far settled. */
    async settled(id: string): Promise<void> {
        await this.#pending.get(id);
    }

    /** Run a socket's send or close after its earlier ones, reporting a failure. */
    #enqueue(id: string, request: RequestInit, url: string): void {
        // send after the socket's earlier sends and closes
        const previous = this.#pending.get(id) ?? Promise.resolve();
        const next = previous.then(async () => {
            const response = await this.#host.fetch(
                new Request(url, { ...request, headers: this.#headers() }),
            );
            await response.arrayBuffer();
            if (!response.ok) {
                throw new CallError("BAD_GATEWAY", {
                    message: `the host refused a hibernated socket's ${request.method}: ${response.status}`,
                });
            }
        });

        // store it as the socket's last, reporting its failure
        const settled = next.catch((error: unknown) => this.#report(error));
        this.#pending.set(id, settled);

        // forget a socket's queue once its last send or close settled
        void settled.then(() => {
            if (this.#pending.get(id) === settled) {
                this.#pending.delete(id);
            }
        });
    }

    /** The headers proving the instance to its host. */
    #headers(): Record<string, string> {
        return { authorization: `Bearer ${this.#secret}` };
    }
}

/** A socket the host hibernates for an instance, with the acceptance the instance answered. */
type HostSocket = ServerSocket<{
    readonly instance: string;
    readonly acceptance: SocketAcceptance;
}>;

/** Hand an instance a request its host makes at a path below its origin. */
export type SocketDelivery = (
    instanceId: string,
    path: string,
    request: RequestInit & { readonly headers: Record<string, string> },
) => Promise<Response>;

/** The host's side of hibernation: the WebSockets it hibernates for its instances. */
export class HibernationHost {
    /** The hibernated sockets, by instance and identifier. */
    readonly #hibernated = new Map<string, Map<string, HostSocket>>();
    /** Hand an instance its hibernated sockets' events. */
    readonly #deliver: SocketDelivery;

    /** Hibernate sockets, delivering their events to their instances. */
    constructor(deliver: SocketDelivery) {
        this.#deliver = deliver;
    }

    /** The handler a listener serving hibernated sockets takes. */
    get websocket(): ServerSocketHandler<HostSocket["attachment"]> {
        return {
            maxPayloadLength: MESSAGE_BYTES,
            open: (socket) => {
                const { instance, acceptance } = socket.attachment;
                this.#upsertSockets(instance).set(acceptance.id, socket);
            },
            message: (socket, message) => void this.#message(socket, message),
            close: (socket) => void this.#close(socket),
        };
    }

    /** Hibernate a WebSocket that an instance accepted in its answer to an upgrade. */
    upgrade(
        listener: Listener<HostSocket["attachment"]>,
        request: Request,
        answer: Response,
    ): boolean {
        // read the instance's acceptance
        const accepted = answer.headers.get(SOCKET_HEADER);
        const instanceId = answer.headers.get(SOCKET_INSTANCE_HEADER);
        if (accepted === null || instanceId === null) {
            throw new CallError("BAD_GATEWAY", {
                message: "the answer has no socket acceptance for a named instance",
            });
        }
        const acceptance = SocketAcceptance.parse(JSON.parse(accepted));

        // attach the instance and its acceptance to the socket
        return listener.upgrade(request, {
            attachment: { instance: instanceId, acceptance },
            ...(acceptance.protocol === undefined
                ? {}
                : { headers: { "sec-websocket-protocol": acceptance.protocol } }),
        });
    }

    /** Answer an instance's listing of its hibernated sockets, or its send or close of one. */
    async answer(instanceId: string, request: Request): Promise<Response> {
        // list every hibernated socket of the instance
        const sockets = this.#sockets(instanceId);
        const { pathname } = new URL(request.url);
        if (pathname === SOCKET_PATH) {
            return Response.json(
                [...sockets.values()].map((socket) =>
                    HibernationHost.#describe(socket.attachment.acceptance),
                ),
            );
        }

        // close the socket of the identifier, a closed one at once, or send to it
        const id = decodeURIComponent(pathname.slice(SOCKET_PATH.length + 1));
        const socket = sockets.get(id);
        if (request.method === "DELETE") {
            socket?.close();
        } else if (socket === undefined) {
            throw new CallError("NOT_FOUND", { message: `no hibernated socket ${id}` });
        } else {
            socket.send(new Uint8Array(await request.arrayBuffer()));
        }

        return new Response(null, { status: 204 });
    }

    /** Close every socket hibernated for an instance, as it stops. */
    release(instanceId: string): void {
        for (const socket of this.#sockets(instanceId).values()) {
            socket.close();
        }
        this.#hibernated.delete(instanceId);
    }

    /** Hand a hibernated socket's message to its instance, answering the declared text itself. */
    async #message(socket: HostSocket, message: string | Uint8Array): Promise<void> {
        // answer the declared text message without the instance
        const { respond } = socket.attachment.acceptance;
        if (respond !== undefined && message === respond.request) {
            socket.send(respond.response);

            return;
        }

        // hand the message over as bytes
        const body =
            typeof message === "string"
                ? new TextEncoder().encode(message)
                : new Uint8Array(message);
        await this.#event(socket, { method: "POST", body });
    }

    /** Hand a hibernated socket's close to its instance and forget the socket. */
    async #close(socket: HostSocket): Promise<void> {
        const { instance, acceptance } = socket.attachment;
        this.#hibernated.get(instance)?.delete(acceptance.id);
        await this.#event(socket, { method: "DELETE" });
    }

    /** Deliver a hibernated socket's event to its instance below the socket's path. */
    async #event(socket: HostSocket, request: RequestInit): Promise<void> {
        // deliver the event below the socket's path, describing the socket
        const { instance, acceptance } = socket.attachment;
        const description = HibernationHost.#describe(acceptance);
        const response = await this.#deliver(
            instance,
            `${SOCKET_PATH}/${encodeURIComponent(acceptance.id)}`,
            { ...request, headers: { [SOCKET_HEADER]: JSON.stringify(description) } },
        );
        await response.arrayBuffer();

        // report a close the instance refuses, which its connection's lapse then sweeps
        if (!response.ok && request.method === "DELETE") {
            instruments.captureException(
                new CallError("BAD_GATEWAY", {
                    message: `instance ${instance} refused the close of socket ${acceptance.id}: ${response.status}`,
                }),
            );
        }
        // close a socket whose message the instance refuses
        else if (!response.ok) {
            socket.close();
        }
    }

    /** Describe an accepted socket as its instance reads it. */
    static #describe(acceptance: SocketAcceptance): SocketDescription {
        return { id: acceptance.id, attachment: acceptance.attachment };
    }

    /** Read the hibernated sockets of an instance, none for an instance without any. */
    #sockets(instanceId: string): ReadonlyMap<string, HostSocket> {
        return this.#hibernated.get(instanceId) ?? new Map();
    }

    /** Read the hibernated sockets of an instance, adding an empty set for its first. */
    #upsertSockets(instanceId: string): Map<string, HostSocket> {
        let sockets = this.#hibernated.get(instanceId);
        if (sockets === undefined) {
            sockets = new Map();
            this.#hibernated.set(instanceId, sockets);
        }

        return sockets;
    }
}
