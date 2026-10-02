import { RetryPolicy } from "@destack/service/timer";
import { Session, type Stream } from "../session/index.ts";

/** How often a host renews its tunnel's token: four times a 60 s token, so a missed renewal still leaves time. */
const HEARTBEAT_MILLISECONDS = 15_000;

/** The status a host answers a forwarded request with when its handler fails without an answer. */
const BAD_GATEWAY = 502;
/** The WebSocket close code refusing a message's data type, from RFC 6455 section 7.4.1. */
const UNSUPPORTED_DATA = 1003;

/** How a host waits before dialing again: half a second, doubling up to 30 seconds, jittered. */
const RETRY = RetryPolicy.of({ initialInterval: 500, maximumInterval: 30_000, jitter: "full" });

/** How a host keeps its tunnel. */
export interface TunnelClientOptions {
    /** The relay's tunnel URL. */
    readonly url: string;
    /** Read the host's current access token for the relay. */
    readonly token: () => Promise<string>;
    /** Answer a request the relay forwards. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** How often to renew the tunnel's token, in milliseconds. */
    readonly heartbeat?: number;
    /** How to wait between attempts, below the policy's interval. */
    readonly retry?: Partial<RetryPolicy>;
    /** Report a failed attempt, such as an unreachable relay or a refused token. */
    readonly report: (error: unknown) => void;
}

/** A host's tunnel to its relay, dialed again with backoff whenever it ends until closed. */
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
        // open the tunnel with the host's token, counting a token the issuer did not grant as a failed dial
        const url = new URL(this.#options.url);
        url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
        let socket: WebSocket;
        try {
            const token = await this.#options.token();
            socket = new WebSocket(url, { headers: { authorization: `Bearer ${token}` } });
        } catch (error) {
            this.#options.report(error);

            return false;
        }
        socket.binaryType = "arraybuffer";

        // close the socket once the client closes
        const stop = () => socket.close();
        this.#stopping.signal.addEventListener("abort", stop, { once: true });
        if (this.#stopping.signal.aborted) {
            stop();
        }

        // open the tunnel once the relay answers the first ping
        const ended = Promise.withResolvers<boolean>();
        let current: Session | undefined;
        let heartbeat: ReturnType<typeof setInterval> | undefined;
        socket.addEventListener("open", () => {
            // carry the session, renewing the token each heartbeat
            const session = new Session(
                { send: (message) => socket.send(message), close: () => socket.close() },
                "client",
                { accept: (stream) => void this.#answer(stream) },
            );
            current = session;
            heartbeat = this.#beat(session, socket);

            // release the waiting callers once the relay answers
            session.ping().then(
                () => {
                    this.#session = session;
                    this.#waiting?.resolve();
                    this.#waiting = undefined;
                },
                () => {},
            );
        });
        socket.addEventListener("message", (event) => {
            // read a binary frame
            if (event.data instanceof ArrayBuffer) {
                current?.receive(new Uint8Array(event.data));
            }
            // refuse a text frame
            else {
                socket.close(UNSUPPORTED_DATA, "the relay sends binary frames only");
            }
        });

        // end the session with the socket
        socket.addEventListener("close", () => {
            // forget the session, noting whether the tunnel opened
            const isOpened = this.#session === current && current !== undefined;
            this.#session = undefined;

            // stop pinging and following the client's close, then end the session's streams
            this.#stopping.signal.removeEventListener("abort", stop);
            clearInterval(heartbeat);
            current?.terminate();
            ended.resolve(isOpened);
        });

        return ended.promise;
    }

    /** Renew the tunnel's token each heartbeat, and drop the socket when the relay stops answering. */
    #beat(session: Session, socket: WebSocket): ReturnType<typeof setInterval> {
        const interval = this.#options.heartbeat ?? HEARTBEAT_MILLISECONDS;
        let isAnswered = true;

        return setInterval(() => {
            // drop a socket with an unanswered last renewal
            if (!isAnswered) {
                socket.terminate();

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

    /** Renew the tunnel's token through a stream to the relay, refusing a failed renewal. */
    async #renew(session: Session): Promise<void> {
        const token = await this.#options.token();
        const response = await session.fetch(
            new Request(this.#options.url, {
                method: "PUT",
                headers: { authorization: `Bearer ${token}` },
            }),
        );
        if (response.status !== 204) {
            throw new Error(
                `relay refused the renewal: ${response.status} ${await response.text()}`,
            );
        }
    }

    /** Answer a request the relay forwards, reporting a handler that failed without an answer. */
    async #answer(stream: Stream): Promise<void> {
        // answer with the handler's response
        const request = stream.request();
        let response: Response;
        try {
            response = await this.#options.fetch(request);
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
