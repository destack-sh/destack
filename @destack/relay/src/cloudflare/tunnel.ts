/// <reference path="./workers.d.ts" />
import { DurableObject } from "cloudflare:workers";
import { DOMAINS, type Domains } from "@destack/host";
import { type Identifier, schema } from "@destack/schema";
import type { TokenVerifier } from "@destack/service/authentication";
import type { Alarm } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { refusal } from "@destack/service/server";
import { PING_MESSAGE, PONG_MESSAGE, type Session, TUNNEL_PROTOCOL } from "../session/index.ts";
import { Admission, Tunnel, type TunnelHost } from "../server/index.ts";

/** The header carrying an admitted machine's admission from the relay's Worker to its object, since a WebSocket upgrade only answers `fetch`. */
const ADMISSION_HEADER = "Destack-Admission";

/** What a machine's WebSocket keeps across its object's hibernation: its admission and when it opened. */
const Attachment = Admission.extend({
    /** When the WebSocket opened, in UTC epoch milliseconds, which orders a machine's connections. */
    openedAt: schema.number(),
});

/** What a machine's WebSocket keeps across its object's hibernation. */
type Attachment = schema.Infer<typeof Attachment>;

/** The server end of a WebSocket workerd keeps open for a Durable Object across hibernation. */
export interface HibernatingSocket {
    /** Send one binary message. */
    send(message: Uint8Array): void;
    /** Close the connection. */
    close(): void;
    /** Keep a value with the WebSocket across hibernation. */
    serializeAttachment(value: unknown): void;
    /** Read the value kept with the WebSocket. */
    deserializeAttachment(): unknown;
}

/** The members of a Durable Object's state a machine's tunnel uses: its hibernating WebSockets, their auto-response and the storage keeping its alarm. */
export interface DurableObjectTunnelState {
    /** The object's storage, whose alarm closes connections as their tokens lapse. */
    readonly storage: Alarm;
    /** Keep a WebSocket open across hibernation, delivering its events to the object's handlers. */
    acceptWebSocket(socket: HibernatingSocket): void;
    /** List the WebSockets kept open, after a wake from hibernation too. */
    getWebSockets(): HibernatingSocket[];
    /** Answer a text message without waking the object. */
    setWebSocketAutoResponse(pair: WebSocketRequestResponsePair): void;
}

/** The members of a machine's object its Worker calls over RPC. */
export interface DurableObjectTunnelStub {
    /** Hand the object an admitted machine's WebSocket upgrade. */
    fetch(request: Request): Promise<Response>;
    /** Forward a request through the machine's tunnel. */
    forward(request: Request): Promise<Response>;
    /** Tell the machine the name the relay routes to it. */
    rename(name: string): Promise<void>;
    /** Close the machine's tunnel, answering whether a connection was open. */
    close(): Promise<boolean>;
}

/** The namespace of the machines' objects, one per machine named by its identifier. */
export interface DurableObjectTunnelNamespace {
    /** Derive an object's identifier from its name. */
    idFromName(name: string): unknown;
    /** Reach an object. */
    get(id: unknown): DurableObjectTunnelStub;
}

/** The pair of WebSocket ends workerd creates for an upgrade, the client's first. */
declare const WebSocketPair: new () => { readonly 0: unknown; readonly 1: HibernatingSocket };

/** A text message and the answer workerd sends to it on a hibernating WebSocket's behalf. */
interface WebSocketRequestResponsePair {
    /** The message. */
    readonly request: string;
    /** The answer. */
    readonly response: string;
}

/** Pair a text message with the answer workerd sends to it. */
declare const WebSocketRequestResponsePair: new (
    request: string,
    response: string,
) => WebSocketRequestResponsePair;

/** What workerd answers an upgrade with: the switch, and the client's end of the WebSocket. */
interface Upgrade extends ResponseInit {
    /** The client's end of the WebSocket. */
    readonly webSocket: unknown;
}

/** How a machine's object keeps its tunnel. */
export interface DurableObjectTunnelOptions {
    /** The relay's tunnel URL machines dial. */
    readonly url: string;
    /** Verify the universe's tokens machines renew their tunnels with, without a database. */
    readonly tokens: Pick<TokenVerifier, "verify">;
    /** Report a failure no request answers. */
    readonly report: (error: unknown) => void;
    /** The domains names resolve under, Destack's by default. */
    readonly domains?: Domains;
}

/** The tunnels of a relay's machines as its Worker reaches them: one Durable Object per machine, named by the machine, called over RPC. */
export class DurableObjectTunnelHost implements TunnelHost {
    /** The namespace of the machines' objects. */
    readonly #namespace: DurableObjectTunnelNamespace;

    /** Reach the machines' objects in a namespace. */
    constructor(namespace: DurableObjectTunnelNamespace) {
        this.#namespace = namespace;
    }

    /** Hand an admitted machine's WebSocket upgrade to its object, the one call RPC cannot carry since its answer holds a WebSocket. */
    open(admission: Admission, request: Request): Promise<Response> {
        const headers = new Headers(request.headers);
        headers.set(ADMISSION_HEADER, JSON.stringify(admission));

        return this.#object(admission.machineId).fetch(new Request(request, { headers }));
    }

    /** Forward a request through a machine's object, which refuses it while the machine keeps no tunnel. */
    fetch(machineId: Identifier<"machine">, request: Request): Promise<Response> {
        return this.#object(machineId).forward(request);
    }

    /** Tell a machine through its object the name the relay routes to it. */
    rename(machineId: Identifier<"machine">, name: string): Promise<void> {
        return this.#object(machineId).rename(name);
    }

    /** Close a machine's tunnel through its object, answering whether a connection was open. */
    close(machineId: Identifier<"machine">): Promise<boolean> {
        return this.#object(machineId).close();
    }

    /** Reach a machine's object. */
    #object(machineId: Identifier<"machine">): DurableObjectTunnelStub {
        return this.#namespace.get(this.#namespace.idFromName(machineId));
    }
}

/** A machine's Durable Object: its tunnel's connections as hibernating WebSockets answering liveness probes asleep, closed on the object's alarm as their tokens lapse. */
export class DurableObjectTunnel extends DurableObject implements DurableObjectTunnelStub {
    /** The object's state. */
    readonly #state: DurableObjectTunnelState;
    /** How the object keeps its tunnel. */
    readonly #options: DurableObjectTunnelOptions;
    /** The session over each open WebSocket. */
    readonly #sessions = new Map<HibernatingSocket, Session>();
    /** The machine's tunnel, absent before its first connection. */
    #tunnel: Tunnel | undefined;

    /** Keep a machine's tunnel, resuming the connections its WebSockets kept across hibernation, whose streams all ended before the object slept. */
    constructor(
        state: DurableObjectTunnelState,
        environment: unknown,
        options: DurableObjectTunnelOptions,
    ) {
        // keep the state and answer liveness probes without waking
        super(state, environment);
        this.#state = state;
        this.#options = options;
        state.setWebSocketAutoResponse(
            new WebSocketRequestResponsePair(PING_MESSAGE, PONG_MESSAGE),
        );

        // resume the open WebSockets oldest first
        const kept = state
            .getWebSockets()
            .map((socket) => ({
                socket,
                attachment: Attachment.parse(socket.deserializeAttachment()),
            }))
            .toSorted((first, second) => first.attachment.openedAt - second.attachment.openedAt);
        for (const { socket, attachment } of kept) {
            this.#attach(socket, attachment);
        }
    }

    /** Accept a machine's WebSocket the Worker admitted. */
    async fetch(request: Request): Promise<Response> {
        const admission = request.headers.get(ADMISSION_HEADER);
        if (admission === null) {
            return refusal(
                new ServiceError("NOT_FOUND", {
                    message: "machines' objects take admissions only",
                }),
            );
        }

        return this.#open(Admission.parse(JSON.parse(admission)));
    }

    /** Forward a request through the machine's tunnel, refusing it while the machine keeps none. */
    async forward(request: Request): Promise<Response> {
        try {
            if (this.#tunnel === undefined) {
                throw new ServiceError("SERVICE_UNAVAILABLE", {
                    message: "the machine keeps no tunnel",
                });
            }

            return await this.#tunnel.fetch(request);
        } catch (error) {
            return refusal(error);
        }
    }

    /** Tell a connected machine its name, keeping it with its WebSockets for later renewals. */
    async rename(name: string): Promise<void> {
        // keep the name across hibernation
        for (const socket of this.#sessions.keys()) {
            const attachment = Attachment.parse(socket.deserializeAttachment());
            socket.serializeAttachment({ ...attachment, name });
        }

        await this.#tunnel?.rename(name);
    }

    /** Close the machine's tunnel, as its revocation does, answering whether a connection was open. */
    async close(): Promise<boolean> {
        return (await this.#tunnel?.close()) ?? false;
    }

    /** Feed a binary message to its connection's session, closing a connection that sends other text than a liveness probe. */
    webSocketMessage(socket: HibernatingSocket, message: ArrayBuffer | string): void {
        // refuse a text frame since workerd answers the liveness probe
        if (typeof message === "string") {
            socket.close();
        }
        // read a binary frame
        else {
            this.#sessions.get(socket)?.receive(new Uint8Array(message));
        }
    }

    /** Forget a closed connection, and its session's streams. */
    async webSocketClose(socket: HibernatingSocket): Promise<void> {
        await this.#detach(socket);
    }

    /** Forget a failed connection, and its session's streams. */
    async webSocketError(socket: HibernatingSocket): Promise<void> {
        await this.#detach(socket);
    }

    /** Close the connections whose tokens lapsed. */
    async alarm(): Promise<void> {
        await this.#tunnel?.lapse();
    }

    /** Accept an admitted machine's WebSocket as the newest connection of its tunnel. */
    #open(admission: Admission): Response {
        // keep the server's end across hibernation with the machine's admission
        const pair = new WebSocketPair();
        const socket = pair[1];
        this.#state.acceptWebSocket(socket);
        const attachment: Attachment = { ...admission, openedAt: Date.now() };
        socket.serializeAttachment(attachment);
        this.#attach(socket, attachment);

        // switch protocols, handing the client's end to the machine
        const upgrade: Upgrade = {
            status: 101,
            headers: { "sec-websocket-protocol": TUNNEL_PROTOCOL },
            webSocket: pair[0],
        };

        return new Response(null, upgrade);
    }

    /** Run a session over a WebSocket in the machine's tunnel, keeping each renewed lapse in the WebSocket's attachment. */
    #attach(socket: HibernatingSocket, attachment: Attachment): void {
        // keep the machine's tunnel on the object's alarm under the newest admission's name
        const { machineId, lapsesAt, name } = attachment;
        this.#tunnel ??= new Tunnel(machineId, name, {
            url: this.#options.url,
            tokens: this.#options.tokens,
            alarm: this.#state.storage,
            report: this.#options.report,
            domains: this.#options.domains ?? DOMAINS,
            keep: (session, renewed) => this.#keep(session, renewed),
        });
        this.#tunnel.name = name;

        // run the session over the socket
        const session = this.#tunnel.attach(
            { send: (message) => socket.send(message), close: () => socket.close() },
            lapsesAt,
        );
        this.#sessions.set(socket, session);
    }

    /** Keep a connection's renewed lapse in its WebSocket's attachment. */
    #keep(session: Session, lapsesAt: number): void {
        for (const [socket, kept] of this.#sessions) {
            if (kept === session) {
                const attachment = Attachment.parse(socket.deserializeAttachment());
                socket.serializeAttachment({ ...attachment, lapsesAt });
            }
        }
    }

    /** Forget a WebSocket's session in the machine's tunnel. */
    async #detach(socket: HibernatingSocket): Promise<void> {
        const session = this.#sessions.get(socket);
        this.#sessions.delete(socket);
        if (session !== undefined) {
            await this.#tunnel?.detach(session);
        }
    }
}
