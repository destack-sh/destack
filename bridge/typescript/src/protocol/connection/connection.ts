import { decodeFrame, encodeFrame } from "../codec.js";
import {
    clientDescriptor,
    protocolLimits,
    protocolRange,
} from "../../_generated/protocol/defaults.js";
import type {
    ClientDescriptor,
    HandshakeResponse,
    ProtocolLimits,
} from "../../_generated/protocol/handshake.js";
import type {
    ProtocolMessage,
    ProtocolNotification,
    RequestOptions,
    ProtocolResponse,
} from "../../_generated/protocol/envelope.js";
import type { PayloadChunkNotification } from "../../_generated/protocol/payload.js";
import { WorkspaceRequest } from "../../_generated/protocol/request.js";
import type { WorkspaceResponse } from "../../_generated/protocol/response.js";
import type { ProtocolRange } from "../../_generated/protocol/version.js";
import { checkedNumber } from "./integer.js";
import { PayloadReceiver } from "./payload/receiver.js";
import type { Transport } from "./transport.js";
import { WebSocketTransport } from "./transport.js";

/** Handler for protocol notifications. */
export type NotificationHandler = (notification: ProtocolNotification) => void;

/** Options for workspace protocol negotiation. */
export type ClientOptions = {
    /** Supported protocol range on the client. */
    readonly protocol?: ProtocolRange;
    /** Requested protocol limits. */
    readonly limits?: ProtocolLimits;
    /** Client descriptor overrides. */
    readonly client?: Partial<ClientDescriptor>;
};

/** Options for one workspace protocol connection. */
export type ConnectionOptions = {
    /** Handle notifications sent by the workspace. */
    readonly onNotification?: NotificationHandler;
    /** Protocol negotiation options. */
    readonly client?: ClientOptions;
};

/** Pending response state for one active protocol request. */
type PendingResponse = {
    readonly resolve: (response: WorkspaceResponse) => void;
    readonly reject: (error: Error) => void;
    readonly payloads: PayloadReceiver;
    response?: WorkspaceResponse;
};

/** Async connection to one workspace protocol endpoint. */
export class Connection {
    readonly #transport: Transport;
    readonly #onNotification: NotificationHandler | undefined;
    readonly #pending = new Map<
        number,
        PendingResponse
    >();
    #nextRequestId = 1;
    #isClosed = false;
    #handshake: HandshakeResponse | undefined;
    #requests = Promise.resolve();

    /** Create one connection over an open binary transport. */
    constructor(transport: Transport, options: ConnectionOptions = {}) {
        this.#transport = transport;
        this.#onNotification = options.onNotification;
        void this.#receive();
    }

    /** Connect to one WebSocket workspace endpoint. */
    static async connectWebSocket(
        url: string | URL,
        options: ConnectionOptions = {},
    ): Promise<Connection> {
        const transport = await WebSocketTransport.connect(url);
        const connection = new Connection(transport, options);
        await connection.handshake(options.client);

        return connection;
    }

    /** Return the negotiated handshake response, if any. */
    get handshakeResponse(): HandshakeResponse | undefined {
        return this.#handshake;
    }

    /** Negotiate the workspace protocol for this connection. */
    async handshake(options: ClientOptions = {}): Promise<HandshakeResponse> {
        if (this.#handshake !== undefined) {
            return this.#handshake;
        }

        const response = await this.request(WorkspaceRequest.handshake({
            protocol: options.protocol ?? protocolRange,
            client: {
                ...clientDescriptor,
                ...options.client,
            },
            limits: options.limits ?? protocolLimits,
        }));

        if (response.kind === "handshake") {
            this.#handshake = response.handshake;

            return response.handshake;
        }

        if (response.kind === "error") {
            throw new Error(response.error.message);
        }

        throw new Error(`unexpected handshake response: ${response.kind}`);
    }

    /** Send one workspace request and wait for its response. */
    async request(
        payload: WorkspaceRequest,
        options: RequestOptions = {},
    ): Promise<WorkspaceResponse> {
        const previous = this.#requests;
        let release: () => void = () => {};
        const previousSettled = previous.catch(ignoreRequestFailure);
        this.#requests = previousSettled.then(() => new Promise<void>((resolve) => {
            release = resolve;
        }));

        await previousSettled;

        try {
            return await this.#request(payload, options);
        } finally {
            release();
        }
    }

    async #request(
        payload: WorkspaceRequest,
        options: RequestOptions,
    ): Promise<WorkspaceResponse> {
        if (this.#isClosed) {
            throw new Error("workspace connection is closed");
        }

        const id = this.#nextRequestId;
        this.#nextRequestId += 1;

        const message: ProtocolMessage = {
            kind: "request",
            request: {
                id: BigInt(id),
                options,
                payload,
            },
        };
        const response = new Promise<WorkspaceResponse>((resolve, reject) => {
            this.#pending.set(id, {
                resolve,
                reject,
                payloads: new PayloadReceiver(),
            });
        });

        try {
            await this.#transport.send(encodeFrame(message));
        } catch (error) {
            this.#pending.delete(id);
            throw error;
        }

        return response;
    }

    /** Close this connection. */
    close(): void {
        this.#isClosed = true;
        this.#transport.close();
        this.#rejectPending(new Error("workspace connection closed"));
    }

    async #receive(): Promise<void> {
        try {
            while (!this.#isClosed) {
                const frame = await this.#transport.receive();
                const message = decodeFrame(frame);
                this.#receiveMessage(message);
            }
        } catch (error) {
            const reason = error instanceof Error ? error : new Error(String(error));
            this.#rejectPending(reason);
        }
    }

    #receiveMessage(message: ProtocolMessage): void {
        if (message.kind === "response") {
            this.#receiveResponse(message.response);
        } else if (message.kind === "notification") {
            this.#receiveNotification(message.notification);
        } else {
            throw new Error("workspace connection received a request");
        }
    }

    #receiveResponse(response: ProtocolResponse): void {
        const id = checkedNumber(response.id, "response id");
        const pending = this.#pending.get(id);
        if (pending === undefined) {
            throw new Error(`workspace connection received unknown response id: ${id}`);
        }

        pending.response = response.payload;
        this.#resolvePending(id, pending);
    }

    #receiveNotification(notification: ProtocolNotification): void {
        if (notification.payload.kind === "payloadChunk") {
            this.#receivePayloadChunk(notification.payload.payload_chunk);
        } else {
            this.#onNotification?.(notification);
        }
    }

    #receivePayloadChunk(chunk: PayloadChunkNotification): void {
        const pending = this.#activePending();
        if (pending === undefined) {
            throw new Error("workspace connection received payload chunk without active request");
        }

        pending.entry.payloads.ingest(chunk);
        if (pending.entry.response !== undefined) {
            this.#resolvePending(pending.id, pending.entry);
        }
    }

    #activePending():
        | {
              readonly id: number;
              readonly entry: PendingResponse;
          }
        | undefined {
        const first = this.#pending.entries().next();
        if (first.done) {
            return undefined;
        }

        const [id, entry] = first.value;

        return { id, entry };
    }

    #resolvePending(id: number, pending: PendingResponse): void {
        if (pending.response === undefined) {
            return;
        }

        const response = pending.payloads.resolve(pending.response);
        if (response === undefined) {
            return;
        }

        this.#pending.delete(id);
        pending.resolve(response);
    }

    #rejectPending(error: Error): void {
        for (const pending of this.#pending.values()) {
            pending.reject(error);
        }

        this.#pending.clear();
    }
}

/** Ignore one earlier request failure while preserving request ordering. */
function ignoreRequestFailure(_error: unknown): void {}

/** Connect to one WebSocket workspace endpoint. */
export function connectEndpoint(
    url: string | URL,
    options: ConnectionOptions = {},
): Promise<Connection> {
    return Connection.connectWebSocket(url, options);
}
