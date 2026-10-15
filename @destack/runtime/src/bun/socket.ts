import type { Server, ServerWebSocket, WebSocketHandler } from "bun";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { CallError } from "../error/index.ts";
import {
    type HeldSocketDescription,
    SOCKET_HEADER,
    SOCKET_INSTANCE_HEADER,
    SOCKET_PATH,
    SocketAcceptance,
} from "../workload/socket.ts";

/** The instruments capturing the closes instances refuse. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The most bytes in one WebSocket message: 1 MiB. */
const MESSAGE_BYTES = 1024 * 1024;

/** Hand an instance a request its host makes at a path below its origin. */
export type SocketDelivery = (
    instanceId: string,
    path: string,
    request: RequestInit & { readonly headers: Record<string, string> },
) => Promise<Response>;

/** What a held WebSocket keeps in Bun. */
interface Held {
    /** The instance holding the socket. */
    readonly instanceId: string;
    /** The socket's identifier and kept value. */
    readonly description: HeldSocketDescription;
    /** A text message the host answers itself, and its answer. */
    readonly respond: SocketAcceptance["respond"];
}

/** The WebSockets a Bun host keeps open for its instances. */
export class BunHibernation {
    /** The held sockets, by instance and identifier. */
    readonly #held = new Map<string, Map<string, ServerWebSocket<Held>>>();
    /** Hand an instance its held sockets' events. */
    readonly #deliver: SocketDelivery;

    /** Keep sockets open, their events going to their instances through a delivery. */
    constructor(deliver: SocketDelivery) {
        this.#deliver = deliver;
    }

    /** The handler a Bun server serving held sockets takes. */
    get websocket(): WebSocketHandler<Held> {
        return {
            maxPayloadLength: MESSAGE_BYTES,
            open: (socket) => {
                const { instanceId, description } = socket.data;
                this.#sockets(instanceId).set(description.id, socket);
            },
            message: (socket, message) => void this.#message(socket, message),
            close: (socket) => void this.#close(socket),
        };
    }

    /** Keep a WebSocket that an instance accepted in its answer to an upgrade. */
    upgrade(server: Server<Held>, request: Request, answer: Response): boolean {
        // read the instance's acceptance
        const accepted = answer.headers.get(SOCKET_HEADER);
        const instanceId = answer.headers.get(SOCKET_INSTANCE_HEADER);
        if (accepted === null || instanceId === null) {
            throw new CallError("BAD_GATEWAY", {
                message: "the answer has no socket acceptance for a named instance",
            });
        }
        const acceptance = SocketAcceptance.parse(JSON.parse(accepted));

        // hold the socket under the identifier the instance gave it, with its kept value
        const description = { id: acceptance.id, attachment: acceptance.attachment };

        return server.upgrade(request, {
            data: { instanceId, description, respond: acceptance.respond },
            ...(acceptance.protocol === undefined
                ? {}
                : { headers: { "sec-websocket-protocol": acceptance.protocol } }),
        });
    }

    /** Answer an instance's listing of its held sockets, or its send or close of one. */
    async answer(instanceId: string, request: Request): Promise<Response> {
        // list every held socket of the instance
        const sockets = this.#sockets(instanceId);
        const { pathname } = new URL(request.url);
        if (pathname === SOCKET_PATH) {
            return Response.json([...sockets.values()].map((socket) => socket.data.description));
        }

        // send to or close the socket of the identifier
        const id = decodeURIComponent(pathname.slice(SOCKET_PATH.length + 1));
        const socket = sockets.get(id);
        if (socket === undefined) {
            throw new CallError("NOT_FOUND", { message: `no held socket ${id}` });
        } else if (request.method === "DELETE") {
            socket.close();
        } else {
            socket.send(new Uint8Array(await request.arrayBuffer()));
        }

        return new Response(null, { status: 204 });
    }

    /** Close every socket held for an instance, as it stops. */
    release(instanceId: string): void {
        for (const socket of this.#sockets(instanceId).values()) {
            socket.close();
        }
        this.#held.delete(instanceId);
    }

    /** Hand a held socket's message to its instance, answering the declared text message itself. */
    async #message(socket: ServerWebSocket<Held>, message: string | Buffer): Promise<void> {
        // answer the declared text message without the instance
        const { respond } = socket.data;
        if (respond !== undefined && message === respond.request) {
            socket.send(respond.response);

            return;
        }

        // hand the message over as bytes
        const body = typeof message === "string" ? new TextEncoder().encode(message) : message;
        await this.#event(socket, { method: "POST", body: new Uint8Array(body) });
    }

    /** Hand a held socket's close to its instance and forget the socket. */
    async #close(socket: ServerWebSocket<Held>): Promise<void> {
        const { instanceId, description } = socket.data;
        this.#sockets(instanceId).delete(description.id);
        await this.#event(socket, { method: "DELETE" });
    }

    /** Deliver a held socket's event to its instance below the socket's path. */
    async #event(socket: ServerWebSocket<Held>, request: RequestInit): Promise<void> {
        // deliver the event below the socket's path, describing the socket
        const { instanceId, description } = socket.data;
        const response = await this.#deliver(
            instanceId,
            `${SOCKET_PATH}/${encodeURIComponent(description.id)}`,
            { ...request, headers: { [SOCKET_HEADER]: JSON.stringify(description) } },
        );
        await response.arrayBuffer();

        // report a close the instance refuses, which its connection's lapse then sweeps
        if (!response.ok && request.method === "DELETE") {
            instruments.captureException(
                new CallError("BAD_GATEWAY", {
                    message: `instance ${instanceId} refused the close of socket ${description.id}: ${response.status}`,
                }),
            );
        }
        // close a socket whose message the instance refuses
        else if (!response.ok) {
            socket.close();
        }
    }

    /** Read the held sockets of an instance. */
    #sockets(instanceId: string): Map<string, ServerWebSocket<Held>> {
        let sockets = this.#held.get(instanceId);
        if (sockets === undefined) {
            sockets = new Map();
            this.#held.set(instanceId, sockets);
        }

        return sockets;
    }
}
