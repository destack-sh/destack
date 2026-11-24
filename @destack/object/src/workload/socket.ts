import { type CallErrorCode, Failure } from "@destack/error";
import { schema, type JsonValue } from "@destack/schema";
import { CallError } from "../error/call.ts";
import { ObjectError } from "../error/error.ts";
import {
    type ClientSocket,
    type Environment,
    MemoryWebSocket,
    type ServerSocket,
    type ServerSocketHandler,
    type ServerUpgrade,
    WebSocketClosure,
    WebSocketUpgrade,
} from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

import type { Fetch } from "../declare/fetch.ts";
import { reportError } from "../error/report.ts";
import { CallFrame } from "../replica/frame.ts";
import type { Voucher } from "../server/http.ts";

/** The path where a host and an instance exchange a hibernated socket's messages and closes. */
export const SOCKET_PATH = "/.destack/socket";

/** The header with a hibernated socket beside its event. */
export const SOCKET_HEADER = "Destack-Socket";

/** The header that identifies the instance that accepted an upgrade. */
export const SOCKET_INSTANCE_HEADER = "Destack-Socket-Instance";

/** The most bytes in one WebSocket message: 1 MiB. */
const MESSAGE_BYTES = 1024 * 1024;

/** The message of a refusal an instance answers a socket event with. */
const RefusalBody = schema.looseObject({ message: schema.string() });

/** The content type of a text message between a host and an instance. */
const TEXT_TYPE = "text/plain;charset=utf-8";

/** The status of an upgrade's answer that switches to the WebSocket (RFC 6455 4.2.2). */
const SWITCHING_STATUS = 101;

/** A hibernated socket as a host describes it: its identifier and the value stored beside it. */
export const SocketDescription = schema.object({
    /** The socket's identifier, unique among the instance's hibernated sockets. */
    id: schema.string().min(1),
    /** The value the instance stored beside the socket when it accepted it. */
    attachment: schema.json(),
    /** The partition of the instance that accepted the socket, absent for its root. */
    partition: schema.string().min(1).exactOptional(),
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
    /** The partition of the instance accepting the socket, absent for its root. */
    partition: schema.string().min(1).exactOptional(),
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
    /** Send one text or binary message in order, settling once the client's socket buffered it. */
    readonly send: (message: string | Uint8Array<ArrayBuffer>) => Promise<void>;
    /** Close the socket. */
    readonly close: () => void;
}

/** What a workload does with its hibernated sockets' messages and closes. */
export interface SocketHandler {
    /** Take a socket once its host keeps it, before its messages, and after the host rebuilt it. */
    readonly open?: (socket: Socket) => Promise<void>;
    /** Take one text or binary message a hibernated socket received. */
    readonly message: (socket: Socket, message: string | Uint8Array<ArrayBuffer>) => Promise<void>;
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

/** How an instance proves itself to the host keeping its sockets. */
export type HibernationProof =
    | {
          /** The secret of an instance in another process. */
          readonly secret: string;
      }
    | {
          /** The name of an instance in the host's own process, which the host trusts. */
          readonly instance: string;
      };

/** The WebSockets a host hibernates for an instance while it sleeps. */
export class Hibernation {
    /** Where the host hibernates the sockets. */
    readonly #host: SocketHost;
    /** How the instance proves itself to its host. */
    readonly #proof: HibernationProof;
    /** The instruments capturing a failed send or close that no caller awaits. */
    readonly #instruments: TelemetryScope;
    /** The last send or close of each socket, which the next one follows. */
    readonly #pending = new Map<string, Promise<void>>();
    /** The partition of the instance whose sockets these are, absent for its root. */
    readonly #partition: string | undefined;

    /** Connect to a partition's sockets at a host as its instance proves itself. */
    constructor(
        host: SocketHost,
        proof: HibernationProof,
        telemetry: Environment["telemetry"],
        partition?: string,
    ) {
        // take the host, the proof, the instruments and the partition
        this.#host = host;
        this.#proof = proof;
        this.#instruments = TelemetryScope.of(import.meta.destack.package, telemetry);
        this.#partition = partition;
    }

    /** Answer a WebSocket upgrade by accepting it, naming an in-process instance beside it. */
    accept(acceptance: SocketAcceptance): Response {
        // stamp the acceptance with the accepting partition
        const partition = this.#partition === undefined ? {} : { partition: this.#partition };
        const accepted = JSON.stringify(SocketAcceptance.parse({ ...acceptance, ...partition }));
        const proof = this.#proof;

        return new Response(null, {
            headers:
                "instance" in proof
                    ? { [SOCKET_HEADER]: accepted, [SOCKET_INSTANCE_HEADER]: proof.instance }
                    : { [SOCKET_HEADER]: accepted },
        });
    }

    /** List the hibernated sockets with their stored values. */
    async list(): Promise<readonly Socket[]> {
        // read the descriptions the host stores
        const response = await this.#host.fetch(
            new Request(this.#host.url, { headers: this.#headers() }),
        );
        if (!response.ok) {
            throw ObjectError.of(
                "HOST_UNREACHABLE",
                `could not list hibernated sockets on the host: status ${response.status}`,
            );
        }

        // take the sockets of this instance's partition alone
        return schema
            .array(SocketDescription)
            .parse(await response.json())
            .filter((description) => description.partition === this.#partition)
            .map((description) => this.socket(description));
    }

    /** Address a hibernated socket the host describes. */
    socket(description: SocketDescription): Socket {
        const url = `${this.#host.url}/${encodeURIComponent(description.id)}`;

        return {
            ...description,
            send: async (message) =>
                this.#enqueue(description.id, SocketMessage.request(message), url),
            close: () => void this.#enqueue(description.id, { method: "DELETE" }, url),
        };
    }

    /** Replace the value stored beside a socket, in order with its sends, as a renewal does. */
    attach(id: string, attachment: JsonValue): void {
        const url = `${this.#host.url}/${encodeURIComponent(id)}`;
        const body = JSON.stringify(attachment);
        void this.#enqueue(id, { method: "PUT", body }, url);
    }

    /** Hand a handler a socket event the host delivered, settling once its sends arrived. */
    async receive(request: Request, handler: SocketHandler): Promise<void> {
        // read the socket the host describes
        const header = request.headers.get(SOCKET_HEADER);
        const description = SocketDescription.parse(JSON.parse(header ?? "null"));
        const socket = this.socket(description);

        // hand over its opening, its close or its message
        if (request.method === "PUT") {
            await handler.open?.(socket);
        } else if (request.method === "DELETE") {
            await handler.close(socket);
        } else {
            await handler.message(socket, await SocketMessage.read(request));
        }

        // settle once the sends the event caused arrived at the host
        await this.settled(description.id);
    }

    /** Wait until a socket's sends and closes so far settled. */
    async settled(id: string): Promise<void> {
        await this.#pending.get(id);
    }

    /** Run a socket's send or close after its earlier ones, settling as it arrived or failed. */
    #enqueue(id: string, request: RequestInit, url: string): Promise<void> {
        // send after the socket's earlier sends and closes
        const previous = this.#pending.get(id) ?? Promise.resolve();
        const next = previous.then(async () => {
            // prove the instance to its host, and refuse a send the host refused
            const headers = new Headers(request.headers);
            for (const [name, value] of Object.entries(this.#headers())) {
                headers.set(name, value);
            }
            const response = await this.#host.fetch(new Request(url, { ...request, headers }));
            await response.arrayBuffer();
            if (!response.ok) {
                throw ObjectError.of(
                    "HOST_UNREACHABLE",
                    `could not '${request.method}' a hibernated socket on the host: status ${response.status}`,
                );
            }
        });

        // store it as the socket's last, capturing its failure
        const settled = next.catch((error: unknown) =>
            this.#instruments.captureException(error, { isEscaped: true }),
        );
        this.#pending.set(id, settled);

        // forget a socket's queue once its last send or close settled
        void settled.then(() => {
            if (this.#pending.get(id) === settled) {
                this.#pending.delete(id);
            }
        });

        return settled;
    }

    /** The headers proving the instance to its host, none for a host in this process. */
    #headers(): Record<string, string> {
        return "secret" in this.#proof ? { authorization: `Bearer ${this.#proof.secret}` } : {};
    }
}

/** What a host keeps beside a socket: the instance that accepted it and its acceptance. */
export const HostSocketAttachment = schema.object({
    /** The instance. */
    instance: schema.string().min(1),
    /** Its acceptance. */
    acceptance: SocketAcceptance,
    /** Whether the host vouches for each of its calls, which a host restart cannot resume. */
    isVouched: schema.boolean(),
});
/** What a host keeps beside a socket. */
export type HostSocketAttachment = schema.Infer<typeof HostSocketAttachment>;

/** A socket the host hibernates for an instance, with the acceptance the instance answered. */
export type HostSocket = ServerSocket<HostSocketAttachment>;

/** What a listener keeping a host's sockets serves: its requests and its sockets' events. */
export interface HostListening {
    /** Answer a request, accepting an upgrade its answer accepts and stating a refusal. */
    readonly fetch: (
        request: Request,
    ) => Promise<Response | ServerUpgrade<HostSocketAttachment | SocketRefusal>>;
    /** Take the kept sockets' events, and state a refused socket's refusal. */
    readonly websocket: ServerSocketHandler<HostSocketAttachment | SocketRefusal>;
}

/** An upgrade's refusal, stated on its socket as one frame and a policy close. */
export class SocketRefusal {
    /** The failure the upgrade's answer states. */
    readonly failure: Failure;

    /** Take the failure a refused upgrade states. */
    private constructor(failure: Failure) {
        this.failure = failure;
    }

    /** Read an answer's refusal of an upgrade, absent for one that switches or accepts a socket. */
    static async of(request: Request, answer: Response): Promise<SocketRefusal | undefined> {
        const isRefused =
            WebSocketUpgrade.of(request) !== undefined &&
            answer.status !== SWITCHING_STATUS &&
            !answer.headers.has(SOCKET_HEADER);

        return isRefused ? new SocketRefusal(await Failure.read(answer)) : undefined;
    }

    /** State the refusal on a socket as it opens: its failure, then a close naming its reason. */
    async send(socket: Pick<ServerSocket<unknown>, "send" | "close">): Promise<void> {
        // send the failure as a call error's body, then close
        const { code, reason, domain, message, data } = this.failure;
        const error = new CallError<CallErrorCode, JsonValue | undefined>(code, {
            reason,
            domain,
            message,
            data,
        });
        await socket.send(JSON.stringify(error));
        socket.close(WebSocketClosure.policy, `upgrade refused: ${reason}`);
    }
}

/** How a host vouches for an accepted socket's calls: the socket's URL and the caller's voucher. */
interface Vouching {
    /** The URL each call is read below. */
    readonly url: string;
    /** The voucher of each call's caller. */
    readonly voucher: Voucher;
}

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
    /** The last delivered event of each socket, which the next one follows in order. */
    readonly #delivering = new Map<HostSocket, Promise<void>>();
    /** Hand an instance its hibernated sockets' events. */
    readonly #deliver: SocketDelivery;
    /** The instruments capturing the closes instances refuse. */
    readonly #instruments: TelemetryScope;
    /** The telemetry the in-process instances' sides capture on. */
    readonly #telemetry: Environment["telemetry"];
    /** The vouchers of answers the host has yet to accept, by instance and socket identifier. */
    readonly #vouching = new Map<string, Vouching>();
    /** The vouchers of accepted sockets, by the attachment each socket keeps until it closes. */
    readonly #vouchers = new WeakMap<HostSocketAttachment, Vouching>();

    /** Hibernate sockets, delivering their events to their instances, captures on a telemetry. */
    constructor(deliver: SocketDelivery, telemetry: Environment["telemetry"]) {
        this.#deliver = deliver;
        this.#telemetry = telemetry;
        this.#instruments = TelemetryScope.of(import.meta.destack.package, telemetry);
    }

    /** The side of an instance in this process, sending through the host directly. */
    instance(id: string): Hibernation {
        return new Hibernation(
            { url: `http://${id}${SOCKET_PATH}`, fetch: (request) => this.answer(id, request) },
            { instance: id },
            this.#telemetry,
        );
    }

    /** The handler a listener serving hibernated sockets takes. */
    get websocket(): ServerSocketHandler<HostSocket["attachment"]> {
        return {
            maxPayloadLength: MESSAGE_BYTES,
            open: (socket) => {
                // keep the socket, then tell its instance before any of its messages
                const { instance, acceptance } = socket.attachment;
                this.#upsertSockets(instance).set(acceptance.id, socket);
                this.#enqueue(socket, () => this.#event(socket, { method: "PUT" }));
            },
            message: (socket, message) =>
                this.#enqueue(socket, () => this.#message(socket, message)),
            close: (socket) => this.#enqueue(socket, () => this.#close(socket)),
        };
    }

    /** Answer a listener's requests through a fetch, keeping the sockets its answers accept. */
    listen(fetch: (request: Request) => Promise<Response>): HostListening {
        const kept = this.websocket;

        return {
            fetch: async (request) => {
                // answer the request
                const answer = await fetch(request);
                const isUpgrade = WebSocketUpgrade.of(request) !== undefined;
                const refusal = await SocketRefusal.of(request, answer);
                // open a refused upgrade's socket to state its refusal
                if (refusal !== undefined) {
                    return { attachment: refusal };
                }
                // keep the socket of an upgrade, passing an acceptance a plain call answers on to its caller
                else if (isUpgrade && answer.headers.has(SOCKET_HEADER)) {
                    return this.accept(answer);
                }

                return answer;
            },
            websocket: {
                maxPayloadLength: MESSAGE_BYTES,
                open: (socket) => {
                    // keep an accepted socket
                    if (HibernationHost.#isAccepted(socket)) {
                        kept.open?.(socket);
                    }
                    // state a refused one's refusal
                    else if (socket.attachment instanceof SocketRefusal) {
                        void socket.attachment.send(socket).catch((error: unknown) => {
                            this.#instruments.captureException(error, { isEscaped: true });
                        });
                    }
                },
                message: (socket, message) => {
                    // deliver an accepted socket's message, dropping a refused one's
                    if (HibernationHost.#isAccepted(socket)) {
                        kept.message(socket, message);
                    }
                },
                close: (socket, code, reason) => {
                    // deliver an accepted socket's close, dropping a refused one's
                    if (HibernationHost.#isAccepted(socket)) {
                        kept.close?.(socket, code, reason);
                    }
                },
            },
        };
    }

    /** Report whether a listener's socket is one an instance accepted. */
    static #isAccepted(
        socket: ServerSocket<HostSocketAttachment | SocketRefusal>,
    ): socket is HostSocket {
        return !(socket.attachment instanceof SocketRefusal);
    }

    /** Vouch for each call of the socket an upgrade's answer accepts, through a voucher. */
    vouch(answer: Response, url: string, voucher: Voucher): void {
        const { instance, acceptance } = HibernationHost.#accepted(answer);
        this.#vouching.set(`${instance} ${acceptance.id}`, { url, voucher });
    }

    /** Accept the WebSocket an instance accepted in its answer, hibernating it by its instance. */
    accept(answer: Response): ServerUpgrade<HostSocket["attachment"]> {
        // attach the instance, its acceptance and whether the host vouches for its calls
        const { instance, acceptance } = HibernationHost.#accepted(answer);
        const key = `${instance} ${acceptance.id}`;
        const vouching = this.#vouching.get(key);
        this.#vouching.delete(key);
        const attachment = { instance, acceptance, isVouched: vouching !== undefined };

        // keep the voucher with the attachment, which lives as long as its socket
        if (vouching !== undefined) {
            this.#vouchers.set(attachment, vouching);
        }

        return {
            attachment,
            ...(acceptance.protocol === undefined
                ? {}
                : { headers: { "sec-websocket-protocol": acceptance.protocol } }),
        };
    }

    /** Read the instance and its acceptance an upgrade's answer names. */
    static #accepted(answer: Response): {
        readonly instance: string;
        readonly acceptance: SocketAcceptance;
    } {
        // require the acceptance and the instance beside it
        const accepted = answer.headers.get(SOCKET_HEADER);
        const instance = answer.headers.get(SOCKET_INSTANCE_HEADER);
        if (accepted === null || instance === null) {
            throw ObjectError.of(
                "ACCEPTANCE_UNREACHABLE",
                "could not read a socket acceptance for a named instance in the response",
            );
        }

        return { instance, acceptance: SocketAcceptance.parse(JSON.parse(accepted)) };
    }

    /** Open a WebSocket in this process: its upgrade through a fetch, its server end kept here. */
    open(fetch: (request: Request) => Promise<Response>, upgrade: WebSocketUpgrade): ClientSocket {
        // connect the client's end to the server's end the host keeps
        let server: HostSocket | undefined;
        const client: MemoryWebSocket = new MemoryWebSocket(
            upgrade.request.url.replace(/^http/u, "ws"),
            (message) => {
                if (server !== undefined) {
                    this.websocket.message(server, message);
                }
            },
            (code, reason) => {
                if (server !== undefined) {
                    this.websocket.close?.(server, code, reason);
                }
                client.closed(code, reason);
            },
        );

        // send the upgrade, and keep the end of the socket the instance accepts
        void fetch(upgrade.request)
            .then(async (answer) => {
                // open a socket the instance refused to state its refusal
                const refusal = await SocketRefusal.of(upgrade.request, answer);
                if (refusal !== undefined) {
                    client.opened("");
                    await refusal.send({
                        send: async (message) => client.received(message),
                        close: (code, reason) =>
                            client.closed(code ?? WebSocketClosure.normal, reason ?? ""),
                    });

                    return;
                }

                // keep the accepted socket's server end under its instance, its sends arriving at the client
                const { attachment } = this.accept(answer);
                const { acceptance } = attachment;
                server = {
                    attachment,
                    send: async (message) => client.received(message),
                    close: (code, reason) =>
                        client.closed(code ?? WebSocketClosure.normal, reason ?? ""),
                };
                this.websocket.open?.(server);
                client.opened(acceptance.protocol ?? "");
            })
            .catch((error: unknown) => {
                // close a socket whose upgrade failed
                this.#instruments.captureException(error, { isEscaped: true });
                client.closed(WebSocketClosure.policy, "upgrade failed");
            });

        return client;
    }

    /** Answer an instance's listing of its sockets, or its send, attach or close of one. */
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

        // drop a call to a closed socket, as a WebSocket discards sends once closed
        const id = decodeURIComponent(pathname.slice(SOCKET_PATH.length + 1));
        const socket = sockets.get(id);
        if (socket === undefined) {
            await request.body?.cancel();
        }
        // close the socket
        else if (request.method === "DELETE") {
            socket.close();
        }
        // replace its stored value
        else if (request.method === "PUT") {
            const attachment = schema.json().parse(await request.json());
            socket.attachment.acceptance = { ...socket.attachment.acceptance, attachment };
        }
        // send to it otherwise, answering once its buffer took the message
        else {
            await socket.send(await SocketMessage.read(request));
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

    /** Deliver a socket's event after its earlier ones, capturing a failed delivery. */
    #enqueue(socket: HostSocket, deliver: () => Promise<void>): void {
        // deliver after the socket's earlier events
        const previous = this.#delivering.get(socket) ?? Promise.resolve();
        const delivered = previous
            .then(deliver)
            .catch((error: unknown) =>
                this.#instruments.captureException(error, { isEscaped: true }),
            );
        this.#delivering.set(socket, delivered);

        // forget a socket's queue once its last event arrived
        void delivered.then(() => {
            if (this.#delivering.get(socket) === delivered) {
                this.#delivering.delete(socket);
            }
        });
    }

    /** Hand a hibernated socket's message to its instance, answering the declared text itself. */
    async #message(socket: HostSocket, message: string | Uint8Array): Promise<void> {
        // drop a message of a socket its instance released, and answer the declared text message without the instance
        const { respond } = socket.attachment.acceptance;
        if (!this.#isKept(socket)) {
            return;
        } else if (respond !== undefined && message === respond.request) {
            await socket.send(respond.response);

            return;
        }

        // hand the message over as it arrived, a call vouched for by the host
        const bytes = typeof message === "string" ? message : new Uint8Array(message);
        const vouched = await this.#vouched(socket, bytes);
        if (vouched !== undefined) {
            await this.#event(socket, SocketMessage.request(vouched));
        }
    }

    /** Vouch for a message's call through its socket's voucher, absent once it closes. */
    async #vouched(
        socket: HostSocket,
        message: string | Uint8Array<ArrayBuffer>,
    ): Promise<string | Uint8Array<ArrayBuffer> | undefined> {
        // pass a message of a socket the host does not vouch for, and a binary one
        const { isVouched } = socket.attachment;
        if (!isVouched || typeof message !== "string") {
            return message;
        }

        // close a socket whose voucher a restart lost, which its client opens again
        const vouching = this.#vouchers.get(socket.attachment);
        if (vouching === undefined) {
            socket.close(WebSocketClosure.restart, "the host restarted: vouch for the calls anew");

            return undefined;
        }

        // set the call's caller as the voucher answers, else its refusal
        const call = CallFrame.request(message, vouching.url);
        if (call === undefined) {
            return message;
        }
        try {
            return CallFrame.forward(message, await vouching.voucher(call));
        } catch (error) {
            return CallFrame.refuse(message, reportError(error, this.#telemetry));
        }
    }

    /** Hand a hibernated socket's close to its instance and forget the socket. */
    async #close(socket: HostSocket): Promise<void> {
        // forget the socket, delivering its close to an instance that still keeps it, a released one needing none
        const isKept = this.#isKept(socket);
        const { instance, acceptance } = socket.attachment;
        this.#hibernated.get(instance)?.delete(acceptance.id);
        if (isKept) {
            await this.#event(socket, { method: "DELETE" });
        }
    }

    /** Report whether the host still keeps a socket for its instance, which a release ends. */
    #isKept(socket: HostSocket): boolean {
        const { instance, acceptance } = socket.attachment;

        return this.#hibernated.get(instance)?.get(acceptance.id) === socket;
    }

    /** Deliver a hibernated socket's event to its instance below the socket's path. */
    async #event(socket: HostSocket, request: RequestInit): Promise<void> {
        // deliver the event below the socket's path, describing the socket
        const { instance, acceptance } = socket.attachment;
        const description = HibernationHost.#describe(acceptance);
        const headers = new Headers(request.headers);
        headers.set(SOCKET_HEADER, JSON.stringify(description));
        const response = await this.#deliver(
            instance,
            `${SOCKET_PATH}/${encodeURIComponent(acceptance.id)}`,
            { ...request, headers: Object.fromEntries(headers) },
        );
        const isJson =
            response.headers.get("content-type")?.startsWith("application/json") === true;
        const answered: unknown = isJson ? await response.json() : await response.arrayBuffer();

        // report a close the instance refuses with its reason, which its connection's lapse then sweeps
        if (!response.ok && request.method === "DELETE") {
            const { data } = RefusalBody.safeParse(answered);
            const refused = `could not close socket '${acceptance.id}' on instance '${instance}': status ${response.status}`;
            this.#instruments.captureException(
                ObjectError.of(
                    "INSTANCE_UNREACHABLE",
                    data === undefined ? refused : `${refused}, refused with '${data.message}'`,
                ),
            );
        }
        // close a socket whose message the instance refuses
        else if (!response.ok) {
            socket.close();
        }
    }

    /** Describe an accepted socket as its instance reads it. */
    static #describe(acceptance: SocketAcceptance): SocketDescription {
        const { id, attachment, partition } = acceptance;

        return partition === undefined ? { id, attachment } : { id, attachment, partition };
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

/** A socket message as a request between a host and an instance: text by its content type. */
export const SocketMessage = {
    /** Send a text or binary message as a request's body. */
    request(message: string | Uint8Array<ArrayBuffer>): RequestInit {
        return typeof message === "string"
            ? {
                  method: "POST",
                  body: message,
                  headers: { "content-type": TEXT_TYPE },
              }
            : { method: "POST", body: message };
    },

    /** Read the text or binary message a request carries. */
    async read(request: Request): Promise<string | Uint8Array<ArrayBuffer>> {
        return request.headers.get("content-type") === TEXT_TYPE
            ? await request.text()
            : new Uint8Array(await request.arrayBuffer());
    },
};
