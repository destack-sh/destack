import { telemetry } from "@destack/telemetry";
import { RetryPolicy } from "@destack/runtime/timer";
import {
    PING_MESSAGE,
    PONG_MESSAGE,
    Renewal,
    Session,
    type Stream,
    TunnelProtocol,
} from "../session/index.ts";
import type { ClientSocket, Environment } from "@destack/environment";

/** The instruments capturing the failed dials, renewals and forwarded requests. */
const instruments = telemetry.scope(import.meta.destack.package);

/** How often a machine probes its tunnel: 15 s, under Cloudflare's 100 s idle WebSocket timeout. */
const HEARTBEAT_MILLISECONDS = 15_000;

/** The share of a token's lifetime a machine waits before renewing it: 80%, as kubelet does. */
const RENEWAL_SHARE = 0.8;

/** The status a machine answers a forwarded request with when its handler gives no answer. */
const BAD_GATEWAY = 502;

/** The WebSocket close code refusing a message's data type, from RFC 6455 section 7.4.1. */
const UNSUPPORTED_DATA = 1003;

/** How a machine waits before dialing again: 0.5 s doubling to 30 s, with AWS's full jitter. */
const RETRY = RetryPolicy.of({ initialInterval: 500, maximumInterval: 30_000, jitter: "full" });

/** How a machine runs its tunnel. */
export interface TunnelClientOptions {
    /** The relay's tunnel URL. */
    readonly url: string;
    /** Read the machine's current access token for the relay. */
    readonly token: () => Promise<string>;
    /** Answer a request the relay forwards. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** Learn the machine's name and its tunnel's lapse at each renewal. */
    readonly renewed: (renewal: Renewal) => void;
    /** How often to probe the tunnel's liveness, in milliseconds. */
    readonly heartbeat?: number;
    /** How to wait between attempts, below the policy's interval. */
    readonly retry?: Partial<RetryPolicy>;
    /** The clock the tunnel waits its heartbeats and retries on, and the network it dials. */
    readonly environment: Environment;
}

/** A machine's tunnel to its relay, dialed again with backoff whenever it ends until closed. */
export class TunnelClient {
    /** How the tunnel runs. */
    readonly #options: TunnelClientOptions;
    /** How to wait between attempts. */
    readonly #retry: RetryPolicy;
    /** How often to probe the tunnel's liveness, in milliseconds. */
    readonly #heartbeat: number;
    /** Stops the dialing loop. */
    readonly #stopping = new AbortController();
    /** The session over the open tunnel, absent while dialing. */
    #session: Session | undefined;
    /** The callers waiting for the tunnel to open, absent while none wait. */
    #waiting: PromiseWithResolvers<void> | undefined;
    /** The dialing loop. */
    #running: Promise<void> | undefined;

    /** Create a tunnel as the options describe. */
    private constructor(options: TunnelClientOptions) {
        this.#options = options;
        this.#retry = RetryPolicy.of({ ...RETRY, ...options.retry });
        this.#heartbeat = options.heartbeat ?? HEARTBEAT_MILLISECONDS;
    }

    /** Dial a relay, dialing again after each end until closed. */
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
                instruments.captureMessage(`tunnel to ${this.#options.url} did not open`, {
                    level: "warning",
                });
            }

            // wait out the jittered backoff
            const isWaited = await RetryPolicy.pause(
                this.#retry,
                failures,
                this.#options.environment.clock,
                this.#stopping.signal,
            );
            if (!isWaited) {
                return;
            }
        }
    }

    /** Dial the relay once and resolve whether the tunnel opened after it ends. */
    async #dial(): Promise<boolean> {
        // connect with the machine's token
        let socket: ClientSocket;
        try {
            socket = await this.#connect();
        } catch (error) {
            // warn of a dial the backoff retries
            instruments.captureMessage(`dialing ${this.#options.url} failed: ${String(error)}`, {
                level: "warning",
            });

            return false;
        }

        // follow the socket and the client's close until the connection ends
        const connection: Connection = {
            socket,
            following: new AbortController(),
            ended: Promise.withResolvers<boolean>(),
            session: undefined,
            renewal: undefined,
            isAlive: true,
        };
        this.#follow(connection);

        return connection.ended.promise;
    }

    /** Open a socket to the relay that offers the machine's token as a subprotocol. */
    async #connect(): Promise<ClientSocket> {
        // address the relay's tunnel over WebSocket
        const url = new URL(this.#options.url);
        url.protocol = url.protocol === "https:" ? "wss:" : "ws:";

        // offer the machine's token
        const token = await this.#options.token();

        return this.#options.environment.network.connect(url.href, TunnelProtocol.offer(token));
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

    /** Start a session over an opened socket. */
    #start(connection: Connection): void {
        // run the session over the socket
        const socket = connection.socket;
        const session = new Session(
            { send: (message) => socket.send(message), close: () => socket.close() },
            "client",
            { accept: (stream) => void this.#answer(session, connection, stream) },
        );
        connection.session = session;

        // probe liveness each heartbeat
        void this.#beat(connection);

        // open the tunnel once the relay answers its first ping
        session.ping().then(
            () => this.#release(connection, session),
            () => {},
        );
    }

    /** Open the tunnel over a session the relay answered, and renew its token near each lapse. */
    #release(connection: Connection, session: Session): void {
        // release the callers
        this.#session = session;
        this.#waiting?.resolve();
        this.#waiting = undefined;

        // learn the machine's name and its token's lifetime
        void this.#schedule(connection, session, 0);
    }

    /** Pass a binary frame to the session, and take a liveness answer. */
    #receive(connection: Connection, event: MessageEvent): void {
        // read a binary frame
        if (event.data instanceof ArrayBuffer) {
            connection.session?.receive(new Uint8Array(event.data));
        }
        // take the answer to a liveness probe
        else if (event.data === PONG_MESSAGE) {
            connection.isAlive = true;
        }
        // refuse other text
        else {
            connection.socket.close(UNSUPPORTED_DATA, "the relay sends binary frames only");
        }
    }

    /** Close the socket and end the connection. */
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

        // stop following the client's close, probing and renewing
        connection.following.abort();
        connection.renewal?.abort();

        // end the session's streams
        session?.terminate();
        connection.ended.resolve(isOpened);
    }

    /** Probe the tunnel's liveness each heartbeat until the connection ends. */
    async #beat(connection: Connection): Promise<void> {
        const { signal } = connection.following;
        while (await this.#sleep(this.#heartbeat, signal)) {
            // drop a socket whose last probe the relay did not answer
            if (!connection.isAlive) {
                this.#drop(connection);

                return;
            }

            // probe again for the relay to answer without waking
            connection.isAlive = false;
            connection.socket.send(PING_MESSAGE);
        }
    }

    /** Renew the tunnel's token after a delay, replacing a scheduled renewal. */
    async #schedule(connection: Connection, session: Session, delay: number): Promise<void> {
        // replace the scheduled renewal, ending with the connection
        connection.renewal?.abort();
        const renewal = new AbortController();
        connection.renewal = renewal;
        const signal = AbortSignal.any([renewal.signal, connection.following.signal]);
        if (!(await this.#sleep(delay, signal))) {
            return;
        }

        // renew again near the renewed token's lapse, or after a heartbeat once it fails
        try {
            const renewed = await this.#renew(session);
            const lifetime = renewed.lapsesAt - this.#options.environment.clock.now();
            void this.#schedule(connection, session, lifetime * RENEWAL_SHARE);
        } catch (error) {
            if (!session.isClosed) {
                instruments.captureException(error);
                void this.#schedule(connection, session, this.#heartbeat);
            }
        }
    }

    /** Wait some milliseconds of the clock, answering false once the signal aborts. */
    async #sleep(milliseconds: number, signal: AbortSignal): Promise<boolean> {
        return this.#options.environment.clock.sleep(milliseconds, signal).then(
            () => true,
            () => false,
        );
    }

    /** Renew the tunnel's token through a stream to the relay, learning the machine's name. */
    async #renew(session: Session): Promise<Renewal> {
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

        // learn the name the relay routes to the machine and the tunnel's lapse
        const renewal = Renewal.parse(await response.json());
        this.#options.renewed(renewal);

        return renewal;
    }

    /** Answer a request the relay forwards. */
    async #answer(session: Session, connection: Connection, stream: Stream): Promise<void> {
        // read the relay's request
        const request = stream.request();
        let response: Response;
        try {
            // renew at once as the relay asks at its own tunnel URL, learning a new name
            if (request.method === "PUT" && request.url === this.#options.url) {
                void this.#schedule(connection, session, 0);
                response = new Response(null, { status: 204 });
            }
            // answer a forwarded request
            else {
                response = await this.#options.fetch(request);
            }
        } catch (error) {
            instruments.captureException(error);
            response = new Response(null, { status: BAD_GATEWAY });
        }

        // send it, and stop the rest of a body the handler left unread unless the relay dropped the stream
        await stream.respond(response);
        if (request.body !== null && !request.bodyUsed && !request.signal.aborted) {
            await request.body.cancel();
        }
    }
}

/** One dial's socket and the session over it. */
interface Connection {
    /** The socket to the relay. */
    readonly socket: ClientSocket;
    /** Stops following the client's close once the connection ends. */
    readonly following: AbortController;
    /** Resolves whether the tunnel opened once the connection ends. */
    readonly ended: PromiseWithResolvers<boolean>;
    /** The session over the socket, absent until the socket opens. */
    session: Session | undefined;
    /** Stops the scheduled token renewal, absent until the tunnel opens. */
    renewal: AbortController | undefined;
    /** Whether the relay answered the last liveness probe. */
    isAlive: boolean;
}
