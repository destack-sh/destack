import { schema } from "@destack/schema";
import { RetryPolicy } from "@destack/service/timer";
import { Session, type Stream, TunnelProtocol } from "../session/index.ts";

/** How often a machine renews its tunnel's token: four times a 60 s token, so a missed renewal still leaves time. */
const HEARTBEAT_MILLISECONDS = 15_000;

/** The status a machine answers a forwarded request with when its handler fails without an answer. */
const BAD_GATEWAY = 502;
/** The WebSocket close code refusing a message's data type, from RFC 6455 section 7.4.1. */
const UNSUPPORTED_DATA = 1003;

/** How a machine waits before dialing again: half a second, doubling up to 30 seconds, jittered. */
const RETRY = RetryPolicy.of({ initialInterval: 500, maximumInterval: 30_000, jitter: "full" });

/** The name the relay routes to the machine, as a renewal answers it or the relay sends it once it changes. */
const MachineName = schema.object({
    /** The name, such as `laptop.florian.destack.computer`. */
    name: schema.string().min(1),
});

/** How a machine keeps its tunnel. */
export interface TunnelClientOptions {
    /** The relay's tunnel URL. */
    readonly url: string;
    /** Read the machine's current access token for the relay. */
    readonly token: () => Promise<string>;
    /** Answer a request the relay forwards. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** Learn the name the relay routes to the machine, once the tunnel opens, at each renewal and once it changes. */
    readonly name: (name: string) => void;
    /** How often to renew the tunnel's token, in milliseconds. */
    readonly heartbeat?: number;
    /** How to wait between attempts, below the policy's interval. */
    readonly retry?: Partial<RetryPolicy>;
    /** Report a failed attempt, such as an unreachable relay or a refused token. */
    readonly report: (error: unknown) => void;
}

/** A machine's tunnel to its relay, dialed again with backoff whenever it ends until closed. */
export class TunnelClient {
    /** How the tunnel is kept. */
    readonly #options: TunnelClientOptions;
    /** How to wait between attempts. */
    readonly #retry: RetryPolicy;
    /** Stops the dialing loop. */
    readonly #stopping = new AbortController();
    /** The session over the open tunnel, absent while dialing. */
    #session: Session | undefined;
    /** The callers waiting for the tunnel to open, absent while none wait. */
    #waiting: PromiseWithResolvers<void> | undefined;
    /** The dialing loop. */
    #running: Promise<void> | undefined;

    /** Keep a tunnel as the options describe. */
    private constructor(options: TunnelClientOptions) {
        this.#options = options;
        this.#retry = RetryPolicy.of({ ...RETRY, ...options.retry });
    }

    /** Dial a relay and keep the tunnel open until closed. */
    static open(options: TunnelClientOptions): TunnelClient {
        const client = new TunnelClient(options);
        client.#running = client.#run();

        return client;
    }

    /** Wait until the tunnel is open and refuse when the client closes first. */
    opened(): Promise<void> {
        // answer at once while open or closed
        if (this.#session !== undefined) {
            return Promise.resolve();
        } else if (this.#stopping.signal.aborted) {
            return Promise.reject(new Error("tunnel client is closed"));
        }

        // wait for the next open
        this.#waiting ??= Promise.withResolvers<void>();

        return this.#waiting.promise;
    }

    /** End the tunnel and stop dialing. */
    async close(): Promise<void> {
        // stop dialing, and refuse the callers still waiting
        this.#stopping.abort();
        this.#waiting?.reject(new Error("tunnel client is closed"));
        this.#waiting = undefined;

        // wait for the loop to end
        await this.#running;
    }

    /** Dial until closed, waiting a jittered delay after each ended tunnel. */
    async #run(): Promise<void> {
        let failures = 0;
        while (!this.#stopping.signal.aborted) {
            // dial, counting failures from the first again after a tunnel that opened
            const isOpened = await this.#dial();
            if (isOpened) {
                failures = 1;
            } else if (!this.#stopping.signal.aborted) {
                failures += 1;
                this.#options.report(new Error(`tunnel to ${this.#options.url} did not open`));
            }

            // wait out the jittered backoff
            const isWaited = await RetryPolicy.pause(this.#retry, failures, this.#stopping.signal);
            if (!isWaited) {
                return;
            }
        }
    }

    /** Dial the relay once and resolve whether the tunnel opened after it ends. */
    async #dial(): Promise<boolean> {
        // connect with the machine's token
        let socket: WebSocket;
        try {
            socket = await this.#connect();
        } catch (error) {
            this.#options.report(error);

            return false;
        }

        // follow the socket and the client's close until the connection ends
        const connection: Connection = {
            socket,
            following: new AbortController(),
            ended: Promise.withResolvers<boolean>(),
            session: undefined,
            heartbeat: undefined,
        };
        this.#follow(connection);

        return connection.ended.promise;
    }

    /** Open a socket to the relay that offers the machine's token as a subprotocol. */
    async #connect(): Promise<WebSocket> {
        // address the relay's tunnel over WebSocket
        const url = new URL(this.#options.url);
        url.protocol = url.protocol === "https:" ? "wss:" : "ws:";

        // offer the machine's token
        const token = await this.#options.token();
        const socket = new WebSocket(url, TunnelProtocol.offer(token));
        socket.binaryType = "arraybuffer";

        return socket;
    }

    /** Listen to the client's close and the socket's events for one connection. */
    #follow(connection: Connection): void {
        // drop the socket when the client closes
        const drop = () => this.#drop(connection);
        this.#stopping.signal.addEventListener("abort", drop, {
            once: true,
            signal: connection.following.signal,
        });
        if (this.#stopping.signal.aborted) {
            drop();
        }

        // run the session over the socket's events
        const socket = connection.socket;
        socket.addEventListener("open", () => this.#start(connection));
        socket.addEventListener("message", (event) => this.#receive(connection, event));
        socket.addEventListener("close", () => this.#end(connection));
    }

    /** Start a session over an opened socket and open the tunnel once the relay answers the first ping. */
    #start(connection: Connection): void {
        // run the session over the socket
        const socket = connection.socket;
        const session = new Session(
            { send: (message) => socket.send(message), close: () => socket.close() },
            "client",
            { accept: (stream) => void this.#answer(stream) },
        );
        connection.session = session;

        // renew the token each heartbeat
        connection.heartbeat = this.#beat(session, () => this.#drop(connection));

        // open the tunnel once the relay answers its first ping
        session.ping().then(
            () => this.#release(session),
            () => {},
        );
    }

    /** Open the tunnel over a session the relay answered and learn the machine's name. */
    #release(session: Session): void {
        // release the callers
        this.#session = session;
        this.#waiting?.resolve();
        this.#waiting = undefined;

        // learn the machine's name, reporting a refusal while the session lasts
        this.#renew(session).catch((error: unknown) => {
            if (!session.isClosed) {
                this.#options.report(error);
            }
        });
    }

    /** Pass a binary frame to the session and close the socket on a text frame. */
    #receive(connection: Connection, event: MessageEvent): void {
        // read a binary frame
        if (event.data instanceof ArrayBuffer) {
            connection.session?.receive(new Uint8Array(event.data));
        }
        // refuse a text frame
        else {
            connection.socket.close(UNSUPPORTED_DATA, "the relay sends binary frames only");
        }
    }

    /** Close the socket and end the connection without waiting for the relay to answer the close. */
    #drop(connection: Connection): void {
        connection.socket.close();
        this.#end(connection);
    }

    /** End the connection once and resolve whether its tunnel opened. */
    #end(connection: Connection): void {
        if (connection.following.signal.aborted) {
            return;
        }

        // forget the session, noting whether the tunnel opened
        const session = connection.session;
        const isOpened = this.#session === session && session !== undefined;
        this.#session = undefined;

        // stop following the client's close
        connection.following.abort();

        // stop renewing the token
        clearInterval(connection.heartbeat);

        // end the session's streams
        session?.terminate();
        connection.ended.resolve(isOpened);
    }

    /** Renew the tunnel's token each heartbeat, and drop the socket when the relay stops answering. */
    #beat(session: Session, drop: () => void): ReturnType<typeof setInterval> {
        const interval = this.#options.heartbeat ?? HEARTBEAT_MILLISECONDS;
        let isAnswered = true;

        return setInterval(() => {
            // drop a socket with an unanswered last renewal
            if (!isAnswered) {
                drop();

                return;
            }

            // renew, dropping the socket at the next beat when the relay refuses or does not answer
            isAnswered = false;
            this.#renew(session).then(
                () => (isAnswered = true),
                (error: unknown) => {
                    if (!session.isClosed) {
                        this.#options.report(error);
                    }
                },
            );
        }, interval);
    }

    /** Renew the tunnel's token through a stream to the relay, learning the machine's name, and refuse a failed renewal. */
    async #renew(session: Session): Promise<void> {
        // renew with the current token
        const token = await this.#options.token();
        const response = await session.fetch(
            new Request(this.#options.url, {
                method: "PUT",
                headers: { authorization: `Bearer ${token}` },
            }),
        );
        if (!response.ok) {
            throw new Error(
                `relay refused the renewal: ${response.status} ${await response.text()}`,
            );
        }

        // learn the name the relay routes to the machine
        this.#options.name(MachineName.parse(await response.json()).name);
    }

    /** Answer a request the relay forwards, or take the name the relay sends to its own tunnel path, reporting a handler that failed without an answer. */
    async #answer(stream: Stream): Promise<void> {
        // read the relay's request
        const request = stream.request();
        let response: Response;
        try {
            // take the name the relay sends to its own tunnel URL
            if (request.method === "PUT" && request.url === this.#options.url) {
                this.#options.name(MachineName.parse(await request.json()).name);
                response = new Response(null, { status: 204 });
            }
            // answer a forwarded request
            else {
                response = await this.#options.fetch(request);
            }
        } catch (error) {
            this.#options.report(error);
            response = new Response(null, { status: BAD_GATEWAY });
        }

        // send it, and stop the rest of a body the handler left unread, leaving a stream the relay dropped
        await stream.respond(response).catch(() => {});
        if (request.body !== null && !request.bodyUsed) {
            await request.body.cancel().catch(() => {});
        }
    }
}

/** One dial's socket and the session over it. */
interface Connection {
    /** The socket to the relay. */
    readonly socket: WebSocket;
    /** Stops following the client's close once the connection ends. */
    readonly following: AbortController;
    /** Resolves whether the tunnel opened once the connection ends. */
    readonly ended: PromiseWithResolvers<boolean>;
    /** The session over the socket, absent until the socket opens. */
    session: Session | undefined;
    /** The token renewal timer, absent until the socket opens. */
    heartbeat: ReturnType<typeof setInterval> | undefined;
}
