import { limits as defaultLimits, peer as defaultPeer, protocolVersion } from "../_generated/rpc/defaults.js";
import type {
    HandshakeRequest,
    HandshakeResponse,
    Limits,
    Peer,
    ServiceOffer,
} from "../_generated/rpc/protocol/handshake.js";
import type { Message } from "../_generated/rpc/protocol/message.js";
import { Call, RpcError, type CallHost } from "./call.js";
import { decodeHandshake, encodeHandshake, encodeValue } from "./codec.js";
import { MessageReceiver, MessageSender, RpcProtocolError, safeLength } from "./payload.js";
import type { Transport } from "./transport.js";
import { WebSocketTransport } from "./transport.js";
import { RpcRequest, type Method, type RequestValue, type RpcResponse } from "./types.js";

/** Negotiation options for one TypeScript RPC connection. */
export type ConnectionOptions = {
    /** Supported exact protocol versions in preference order. */
    readonly versions?: readonly number[];
    /** Advertised connection limits. */
    readonly limits?: Limits;
    /** Informational client peer description. */
    readonly peer?: Peer;
};

/** One negotiated RPC connection to a service peer. */
export class Connection implements CallHost {
    readonly #transport: Transport;
    readonly #options: Required<ConnectionOptions>;
    readonly #calls = new Map<bigint, CallRoute>();
    #sender: MessageSender | undefined;
    #receiver: MessageReceiver | undefined;
    #limits: Limits | undefined;
    #peer: Peer | undefined;
    #services: readonly ServiceOffer[] = [];
    #nextCall = 1n;
    #isClosed = false;
    #isReceiving = false;

    /** Create one unnegotiated connection over an open transport. */
    constructor(transport: Transport, options: ConnectionOptions = {}) {
        this.#transport = transport;
        this.#options = {
            versions: options.versions ?? [protocolVersion],
            limits: options.limits ?? defaultLimits,
            peer: options.peer ?? defaultPeer,
        };
        Connection.#validateLimits(this.#options.limits);
    }

    /** Connect and negotiate one RPC WebSocket. */
    static async connectWebSocket(
        url: string | URL,
        services: readonly bigint[],
        options: ConnectionOptions = {},
    ): Promise<Connection> {
        const transport = await WebSocketTransport.connect(url);
        let connection: Connection;
        try {
            connection = new Connection(transport, options);
        } catch (error) {
            transport.close();

            throw error;
        }
        await connection.handshake(services);

        return connection;
    }

    /** Return the negotiated resource limits. */
    get limits(): Limits {
        if (this.#limits === undefined) {
            throw new RpcProtocolError("RPC connection is not negotiated");
        }

        return this.#limits;
    }

    /** Return the accepting peer description. */
    get peer(): Peer {
        if (this.#peer === undefined) {
            throw new RpcProtocolError("RPC connection is not negotiated");
        }

        return this.#peer;
    }

    /** Negotiate exact protocol and service contracts. */
    async handshake(services: readonly bigint[]): Promise<HandshakeResponse> {
        if (this.#sender !== undefined) {
            throw new RpcProtocolError("RPC connection is already negotiated");
        }

        try {
            const request: HandshakeRequest = {
                versions: this.#options.versions,
                limits: this.#options.limits,
                peer: this.#options.peer,
                services,
            };
            const bytes = encodeHandshake({ kind: "request", request });
            const initialLimit = safeLength(
                this.#options.limits.maxMessageBytes,
                "message limit",
            );
            if (bytes.length > initialLimit) {
                throw new RpcProtocolError(
                    `RPC handshake exceeds ${initialLimit} bytes: ${bytes.length}`,
                );
            }

            await this.#transport.send(bytes);
            const responseBytes = await this.#transport.receive();
            if (responseBytes.length > initialLimit) {
                throw new RpcProtocolError(
                    `RPC handshake exceeds ${initialLimit} bytes: ${responseBytes.length}`,
                );
            }
            const handshake = decodeHandshake(responseBytes);
            if (handshake.kind !== "response") {
                throw new RpcProtocolError("expected RPC handshake response");
            }
            const response = handshake.response;
            if (response.kind === "rejected") {
                throw new RpcError(response.status);
            }

            this.#acceptHandshake(response, services);

            return response;
        } catch (error) {
            const failure = error instanceof Error ? error : new Error(String(error));
            this.#terminate(failure);

            throw failure;
        }
    }

    /** Start one statically typed RPC call. */
    start<Request, Response, Input, Output>(
        method: Method<Request, Response, Input, Output>,
        request: RequestValue<Request>,
    ): Call<Response, Input, Output> {
        const limits = this.limits;
        if (this.#isClosed) {
            throw new RpcProtocolError("RPC connection is closed");
        }
        if (!this.#isMethodOffered(method)) {
            throw new RpcProtocolError("RPC method was not offered by the peer");
        }
        if (this.#calls.size >= limits.maxConcurrentCalls) {
            throw new RpcProtocolError("RPC connection reached its concurrent call limit");
        }

        const rpcRequest = request instanceof RpcRequest ? request : new RpcRequest(request);
        const payload = encodeValue(method.request, rpcRequest.value);
        this.validatePayload(payload);

        const id = this.#nextCall;
        this.#nextCall += 1n;
        const call = new Call<Response, Input, Output>(
            this,
            id,
            method as unknown as Method<unknown, Response, Input, Output>,
            limits.streamWindow,
        );
        this.#calls.set(id, {
            call: call as Call<unknown, unknown, unknown>,
            outputWindow: limits.streamWindow,
        });

        const message: Message = {
            kind: "start",
            start: {
                call: id,
                service: method.service,
                method: method.method,
                request: {
                    metadata: rpcRequest.metadata,
                    value: { kind: "inline", inline: payload },
                },
            },
        };
        void this.send(message).catch(() => {
            // connection termination delivers this failure through the returned call
        });

        return call;
    }

    /** Execute one unary RPC call. */
    async call<Request, Response>(
        method: Method<Request, Response>,
        request: RequestValue<Request>,
    ): Promise<RpcResponse<Response>> {
        const call = this.start(method, request);

        return call.response();
    }

    /** Bind one generated method descriptor to its exact offered contract. */
    bind(method: {
        readonly service: bigint;
        readonly method: bigint;
        readonly fingerprint: bigint;
    }): void {
        if (!this.#isMethodOffered(method)) {
            throw new RpcProtocolError(
                `RPC method ${method.service}/${method.method} was not offered with its expected contract`,
            );
        }
    }

    /** Validate one encoded call value before changing connection state. */
    validatePayload(payload: Uint8Array): void {
        const limit = safeLength(this.limits.maxPayloadBytes, "payload limit");
        if (payload.length > limit) {
            throw new RpcProtocolError(`RPC payload exceeds ${limit} bytes: ${payload.length}`);
        }
    }

    /** Send one negotiated call message. */
    async send(message: Message): Promise<void> {
        if (this.#sender === undefined) {
            throw new RpcProtocolError("RPC connection is not negotiated");
        }

        try {
            await this.#sender.send(message);
        } catch (error) {
            const failure = error instanceof Error ? error : new Error(String(error));
            this.#terminate(failure);

            throw failure;
        }
    }

    /** Release one consumed output item. */
    async releaseOutput(call: bigint): Promise<void> {
        const route = this.#calls.get(call);
        if (route === undefined) {
            return;
        }
        if (route.outputWindow === 0xffffffff) {
            throw new RpcProtocolError("RPC output stream window overflowed");
        }

        route.outputWindow += 1;
        await this.send({ kind: "window", window: { call, items: 1 } });
    }

    /** Close this connection and fail every active call. */
    close(): void {
        this.#terminate(new RpcProtocolError("RPC connection closed"));
    }

    /** Commit one accepted handshake and start message routing. */
    #acceptHandshake(
        response: Extract<HandshakeResponse, { kind: "accepted" }>,
        requested: readonly bigint[],
    ): void {
        if (!this.#options.versions.includes(response.version)) {
            throw new RpcProtocolError("peer selected an unsupported RPC protocol version");
        }
        Connection.#validateLimits(response.limits);
        Connection.#validateServices(response.services, requested);

        this.#limits = response.limits;
        this.#peer = response.peer;
        this.#services = response.services;
        this.#sender = new MessageSender(this.#transport, response.limits);
        this.#receiver = new MessageReceiver(this.#transport, response.limits);
        this.#isReceiving = true;
        void this.#receive();
    }

    /** Receive and route messages until the connection terminates. */
    async #receive(): Promise<void> {
        try {
            while (this.#isReceiving) {
                const receiver = this.#receiver;
                if (receiver === undefined) {
                    throw new RpcProtocolError("RPC message receiver is not initialized");
                }

                const message = await receiver.receive();
                this.#route(message);
            }
        } catch (error) {
            const failure = error instanceof Error ? error : new Error(String(error));

            this.#terminate(failure);
        }
    }

    /** Route one decoded service message. */
    #route(message: Message): void {
        if (message.kind === "item") {
            const route = this.#active(message.item.call);
            if (route.outputWindow === 0) {
                throw new RpcProtocolError("peer exceeded RPC output stream window");
            }

            route.outputWindow -= 1;
            route.call.pushOutput(message.item.payload);
        } else if (message.kind === "window") {
            this.#active(message.window.call).call.updateInputWindow(message.window.items);
        } else if (message.kind === "complete") {
            const call = message.complete.call;
            const route = this.#active(call);
            this.#calls.delete(call);
            route.call.complete(message.complete);
        } else if (message.kind === "cancel") {
            const route = this.#active(message.cancel.call);
            this.#calls.delete(message.cancel.call);
            route.call.canceled();
        } else {
            throw new RpcProtocolError(`service peer sent unexpected RPC ${message.kind} message`);
        }
    }

    /** Return one active call route. */
    #active(call: bigint): CallRoute {
        const route = this.#calls.get(call);
        if (route === undefined) {
            throw new RpcProtocolError(`RPC message references inactive call ${call}`);
        }

        return route;
    }

    /** Return whether the peer selected this exact method contract. */
    #isMethodOffered(method: { readonly service: bigint; readonly method: bigint; readonly fingerprint: bigint }): boolean {
        const service = this.#services.find((offer) => offer.service === method.service);

        return service?.methods.some(
            (offer) => offer.method === method.method && offer.fingerprint === method.fingerprint,
        ) === true;
    }

    /** Permanently fail this connection and every active call. */
    #fail(error: Error): void {
        if (this.#isClosed) {
            return;
        }

        this.#isClosed = true;
        this.#isReceiving = false;
        for (const route of this.#calls.values()) {
            route.call.fail(error);
        }
        this.#calls.clear();
    }

    /** Close the transport and fail every active call. */
    #terminate(error: Error): void {
        if (this.#isClosed) {
            return;
        }

        this.#fail(error);
        this.#transport.close();
    }

    /** Validate all negotiated resource limits. */
    static #validateLimits(limits: Limits): void {
        safeLength(limits.maxMessageBytes, "message limit");
        safeLength(limits.maxPayloadBytes, "payload limit");
        if (limits.maxMessageBytes === 0n || limits.maxPayloadBytes === 0n) {
            throw new RpcProtocolError("RPC byte limits must be positive");
        }
        if (limits.maxConcurrentCalls <= 0 || limits.streamWindow <= 0) {
            throw new RpcProtocolError("RPC call and stream limits must be positive");
        }
    }

    /** Validate the peer's exact requested service and method set. */
    static #validateServices(offers: readonly ServiceOffer[], requested: readonly bigint[]): void {
        const requestedServices = new Set(requested);
        if (requestedServices.size !== offers.length) {
            throw new RpcProtocolError("peer returned an unexpected RPC service offer count");
        }

        const offeredServices = new Set<bigint>();
        for (const offer of offers) {
            if (!requestedServices.has(offer.service) || offeredServices.has(offer.service)) {
                throw new RpcProtocolError(
                    "peer returned an unexpected or duplicate RPC service offer",
                );
            }
            offeredServices.add(offer.service);

            const methods = new Set(offer.methods.map((method) => method.method));
            if (methods.size !== offer.methods.length) {
                throw new RpcProtocolError("peer returned a duplicate RPC method offer");
            }
        }
    }
}

/** Type-erased routing state for one active call. */
type CallRoute = {
    /** Active typed call. */
    readonly call: Call<unknown, unknown, unknown>;
    /** Remaining peer output window. */
    outputWindow: number;
};
