import { defineSchema, schema, type JsonValue } from "@destack/schema";
import { CallError } from "../error/index.ts";
import type { Fetch } from "../runtime/fetch.ts";

/** The path at which a host and an instance exchange a held socket's messages, closes and sends. */
export const SOCKET_PATH = "/.destack/socket";

/** The header with a held socket beside its event. */
export const SOCKET_HEADER = "Destack-Socket";

/** The header that identifies the instance that accepted an upgrade. */
export const SOCKET_INSTANCE_HEADER = "Destack-Socket-Instance";

/** A held socket as a host describes it: its identifier and the value kept beside it. */
export const HeldSocketDescription = defineSchema(
    schema.object({
        /** The socket's identifier, unique among the instance's held sockets. */
        id: schema.string().min(1),
        /** The value the instance kept beside the socket when it accepted it. */
        attachment: schema.json(),
    }),
);
/** A held socket as a host describes it. */
export type HeldSocketDescription = schema.Infer<typeof HeldSocketDescription>;

/** An instance's acceptance of a WebSocket upgrade. */
export const SocketAcceptance = defineSchema(
    schema.object({
        /** The socket's identifier, which the instance picks, unique among its held sockets. */
        id: schema.string().min(1),
        /** The value kept beside the socket. */
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
    }),
);
/** An instance's acceptance of a WebSocket upgrade. */
export type SocketAcceptance = schema.Infer<typeof SocketAcceptance>;

/** One WebSocket a host holds for an instance, open while the instance sleeps. */
export interface HeldSocket {
    /** The socket's identifier. */
    readonly id: string;
    /** The value the instance kept beside the socket. */
    readonly attachment: JsonValue;
    /** Send one binary message, in order with the socket's other sends. */
    readonly send: (message: Uint8Array<ArrayBuffer>) => void;
    /** Close the socket. */
    readonly close: () => void;
}

/** What a workload does with its held sockets' messages and closes. */
export interface SocketHandler {
    /** Take one binary message a held socket received. */
    readonly message: (socket: HeldSocket, message: Uint8Array<ArrayBuffer>) => Promise<void>;
    /** Take the close of a held socket. */
    readonly close: (socket: HeldSocket) => Promise<void>;
}

/** Where an instance connects to the sockets its host keeps open. */
export interface SocketHost {
    /** The URL of the held sockets. */
    readonly url: string;
    /** Send a request to the host. */
    readonly fetch: Fetch;
}

/** The WebSockets a host keeps open for an instance while it sleeps. */
export class Hibernation {
    /** Where the host keeps the sockets. */
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

    /** List the held sockets with their kept values. */
    async list(): Promise<readonly HeldSocket[]> {
        // read the descriptions the host keeps
        const response = await this.#host.fetch(
            new Request(this.#host.url, { headers: this.#headers() }),
        );
        if (!response.ok) {
            throw new CallError("BAD_GATEWAY", {
                message: `the host cannot list held sockets: ${response.status}`,
            });
        }

        return schema
            .array(HeldSocketDescription)
            .parse(await response.json())
            .map((description) => this.socket(description));
    }

    /** Address a held socket the host describes. */
    socket(description: HeldSocketDescription): HeldSocket {
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
                    message: `the host refused a held socket's ${request.method}: ${response.status}`,
                });
            }
        });

        // keep it as the socket's last, reporting its failure
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
