import { Bearer } from "@destack/access/token";
import {
    BearerProtocol,
    type ClientSocket,
    type Environment,
    WebSocketMessage,
    WebSocketUpgrade,
} from "@destack/environment";
import { refusal, reportError } from "@destack/object";
import {
    type HibernationHost,
    SOCKET_HEADER,
    SOCKET_INSTANCE_HEADER,
    SOCKET_PATH,
    SocketAcceptance,
    SocketMessage,
} from "@destack/object/workload";
import { Identifier, present, schema } from "@destack/schema";
import { MachineError } from "../error/index.ts";

/** The WebSockets a machine forwards to other origins, each paired with one it opened there. */
export class SocketBridge {
    /** The sockets opened at the other origins, by the bridge each client's socket is under. */
    readonly #opened = new Map<string, ClientSocket>();
    /** How each bridge signs its client's text frames, by bridge, absent for unsigned frames. */
    readonly #signing = new Map<string, (frame: string) => Promise<string>>();
    /** The host keeping the clients' sockets. */
    readonly #host: Pick<HibernationHost, "answer">;
    /** The network the sockets open through, and the randomness naming the bridges. */
    readonly #environment: Environment;

    /** Bridge sockets kept at a host, opening the other ends through an environment's network. */
    constructor(host: Pick<HibernationHost, "answer">, environment: Environment) {
        this.#host = host;
        this.#environment = environment;
    }

    /** Report whether an instance a host keeps sockets under is a bridge. */
    static is(instance: string): boolean {
        return schema.identifier("bridge").safeParse(instance).success;
    }

    /** Open an upgrade's socket at its origin, answering once it opens, signing client frames. */
    async forward(upgrade: Request, sign?: (frame: string) => Promise<string>): Promise<Response> {
        // offer the upgrade's subprotocols and its bearer token, which a socket cannot send as a header
        const offered = present(
            WebSocketUpgrade.of(upgrade),
            "the forwarded WebSocket upgrade",
        ).protocols;
        const token = Bearer.token(upgrade.headers.get("authorization"));
        const protocols = [
            ...offered,
            ...(token === undefined ? [] : [BearerProtocol.write(token)]),
        ];
        const target = upgrade.url.replace(/^http/u, "ws");
        const opened = this.#environment.network.websocket(target, protocols);

        // answer once it opens, refusing one that closes first
        const isOpen = await new Promise<boolean>((resolve) => {
            opened.addEventListener("open", () => resolve(true), { once: true });
            opened.addEventListener("close", () => resolve(false), { once: true });
        });
        if (!isOpen) {
            return refusal(
                MachineError.of(
                    "SOCKET_UNREACHABLE",
                    `could not open a socket to '${new URL(target).host}'`,
                ),
                this.#environment.telemetry,
            );
        }

        // relay the other end's messages and close to the client's socket in order, reporting a failed relay
        const id = Identifier.create("bridge", this.#environment);
        const url = `http://bridge${SOCKET_PATH}/${encodeURIComponent(id)}`;
        let relaying = Promise.resolve();
        const deliver = (request: RequestInit) => {
            relaying = relaying
                .then(async () => {
                    await this.#host.answer(id, new Request(url, request));
                })
                .catch((error: unknown) => {
                    reportError(error, this.#environment.telemetry);
                });
        };
        opened.addEventListener("message", (event) =>
            deliver(SocketMessage.request(WebSocketMessage.of(event.data))),
        );
        opened.addEventListener("close", () => {
            this.#opened.delete(id);
            this.#signing.delete(id);
            deliver({ method: "DELETE" });
        });
        this.#opened.set(id, opened);
        if (sign !== undefined) {
            this.#signing.set(id, sign);
        }

        // accept the client's socket under the bridge, with the subprotocol the other end chose
        const acceptance = SocketAcceptance.parse({
            id,
            attachment: {},
            ...(opened.protocol === "" ? {} : { protocol: opened.protocol }),
        });

        return new Response(null, {
            headers: {
                [SOCKET_HEADER]: JSON.stringify(acceptance),
                [SOCKET_INSTANCE_HEADER]: id,
            },
        });
    }

    /** Pass a client socket's message or close on to the other end of its bridge. */
    async receive(instance: string, request: Request): Promise<void> {
        // drop an event of a bridge whose other end closed, as a closed WebSocket drops sends, and the host's opening
        const opened = this.#opened.get(instance);
        if (opened === undefined || request.method === "PUT") {
            await request.body?.cancel();
        }
        // close the other end
        else if (request.method === "DELETE") {
            this.#opened.delete(instance);
            this.#signing.delete(instance);
            opened.close();
        }
        // send the message on, signing a text frame as its bridge asks
        else {
            const message = await SocketMessage.read(request);
            const sign = this.#signing.get(instance);
            opened.send(
                sign === undefined || typeof message !== "string" ? message : await sign(message),
            );
        }
    }
}
