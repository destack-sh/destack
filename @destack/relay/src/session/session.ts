import { Frame, FrameFlag, FrameType, GoAwayCode } from "./frame.ts";
import { Head, RequestHead } from "./head.ts";
import { Stream } from "./stream.ts";

/** The most streams a session keeps open each way: 100, the usual HTTP/2 concurrent stream limit. */
export const MAX_STREAMS = 100;

/** The WebSocket, or any other message channel, a session sends its frames over. */
export interface Transport {
    /** Send one frame as one binary message. */
    send(message: Uint8Array<ArrayBuffer>): void;
    /** Close the channel. */
    close(): void;
}

/** Which end of the channel a session is: the dialing side opens odd streams. */
export type SessionRole = "client" | "server";

/** What a session does with its peer's streams. */
export interface SessionOptions {
    /** Take a stream the peer opened. */
    readonly accept: (stream: Stream) => void;
}

/** Streams multiplexed over one message channel, as yamux multiplexes them over one connection. */
export class Session {
    /** The channel carrying the frames. */
    readonly #transport: Transport;
    /** What to do with the peer's streams and pings. */
    readonly #options: SessionOptions;
    /** The open streams, by identifier. */
    readonly #streams = new Map<number, Stream>();
    /** The identifier of the next stream this side opens. */
    #next: number;
    /** The pings waiting for their answer, by value. */
    readonly #pings = new Map<number, PromiseWithResolvers<void>>();
    /** The value of the next ping. */
    #ping = 0;
    /** Whether the session ended. */
    #isClosed = false;

    /** Keep a session over a channel, as the side that dialed or accepted it. */
    constructor(transport: Transport, role: SessionRole, options: SessionOptions) {
        this.#transport = transport;
        this.#options = options;
        this.#next = role === "client" ? 1 : 2;
    }

    /** Whether the session ended. */
    get isClosed(): boolean {
        return this.#isClosed;
    }

    /** How many streams are open. */
    get size(): number {
        return this.#streams.size;
    }

    /** Open a stream forwarding a request's head. */
    open(head: RequestHead): Stream {
        // refuse streams on an ended session
        if (this.#isClosed) {
            throw new TypeError("session is closed");
        }

        // announce the stream with the request's head
        const stream = new Stream(this, this.#next, head);
        this.#next += 2;
        this.#streams.set(stream.id, stream);
        const payload = new TextEncoder().encode(JSON.stringify(head));
        this.send(new Frame(FrameType.data, FrameFlag.syn, stream.id, 0, payload));

        return stream;
    }

    /** Forward a request to the peer over a new stream, answering with the peer's response. */
    async fetch(request: Request): Promise<Response> {
        // open the stream, resetting it once the request aborts
        const stream = this.open(Head.request(request));
        const abort = () => stream.reset(new Error("the request was aborted"));
        request.signal.addEventListener("abort", abort, { once: true });

        // send the body, resetting the stream on a failed read
        const sending =
            request.body === null ? stream.writable.close() : request.body.pipeTo(stream.writable);
        sending.catch((error: unknown) =>
            stream.reset(error instanceof Error ? error : new Error(String(error))),
        );

        // answer with the peer's head and the stream's bytes
        const head = await stream.answer;
        const body = Head.hasResponseBody(head) ? stream.readable : null;

        return new Response(body, { status: head.status, headers: Head.headers(head) });
    }

    /** Probe the peer and resolve on an answer or reject when the session ends first. */
    ping(): Promise<void> {
        // refuse pings on an ended session
        if (this.#isClosed) {
            return Promise.reject(new Error("session is closed"));
        }

        // send a value the answer names
        const answer = Promise.withResolvers<void>();
        const value = this.#ping++ >>> 0;
        this.#pings.set(value, answer);
        this.send(new Frame(FrameType.ping, FrameFlag.syn, 0, value));

        return answer.promise;
    }

    /** Take one message from the channel, ending the session when it breaks the protocol. */
    receive(message: Uint8Array): void {
        // dispatch the frame, ending the session on a protocol violation
        let accepted: Stream | undefined;
        try {
            accepted = this.#receive(Frame.decode(message));
        } catch (error) {
            if (!(error instanceof TypeError)) {
                throw error;
            }
            this.close(GoAwayCode.protocol);
        }

        // hand a stream the peer opened over, outside the protocol's failures
        if (accepted !== undefined) {
            this.#options.accept(accepted);
        }
    }

    /** Send a frame, unless the session ended. */
    send(frame: Frame): void {
        if (!this.#isClosed) {
            this.#transport.send(frame.encode());
        }
    }

    /** Forget a destroyed stream. */
    forget(id: number): void {
        this.#streams.delete(id);
    }

    /** End the session: tell the peer, destroy every stream and close the channel. */
    close(code: number = GoAwayCode.normal): void {
        this.send(new Frame(FrameType.goAway, 0, 0, code));
        this.terminate();
        this.#transport.close();
    }

    /** Destroy every stream once the channel closed. */
    terminate(): void {
        // refuse further frames, fail unanswered pings, and end every stream
        this.#isClosed = true;
        for (const answer of this.#pings.values()) {
            answer.reject(new Error("session is closed"));
        }
        this.#pings.clear();
        for (const stream of this.#streams.values()) {
            stream.destroy(new Error("session closed"));
        }
        this.#streams.clear();
    }

    /** Dispatch a frame to the session or its stream, returning a stream the peer opened. */
    #receive(frame: Frame): Stream | undefined {
        // answer a ping
        if (frame.type === FrameType.ping && !frame.has(FrameFlag.ack)) {
            this.send(new Frame(FrameType.ping, FrameFlag.ack, 0, frame.value));
        }
        // resolve an answered ping
        else if (frame.type === FrameType.ping) {
            this.#pings.get(frame.value)?.resolve();
            this.#pings.delete(frame.value);
        }
        // end on the peer's go-away
        else if (frame.type === FrameType.goAway) {
            this.terminate();
            this.#transport.close();
        }
        // open a stream the peer opens
        else if (frame.has(FrameFlag.syn)) {
            return this.#open(frame);
        }
        // hand stream frames to their stream, dropping late frames of streams already gone
        else {
            this.#streams.get(frame.stream)?.receive(frame);
        }

        return undefined;
    }

    /** Keep a stream the peer opened with a SYN, refuse identifiers of this side or in use, and reset one beyond the limit. */
    #open(frame: Frame): Stream | undefined {
        // refuse identifiers the peer may not open
        const isOwn = frame.stream % 2 === this.#next % 2;
        if (frame.stream === 0 || isOwn || this.#streams.has(frame.stream)) {
            throw new TypeError(`peer opened stream ${frame.stream}, which it may not open`);
        }
        // reset a stream beyond the open streams a session keeps
        else if (this.#streams.size >= MAX_STREAMS) {
            this.send(new Frame(FrameType.window, FrameFlag.rst, frame.stream));

            return undefined;
        }

        // keep the stream under the request head it carries
        const stream = new Stream(this, frame.stream, readHead(frame.payload));
        this.#streams.set(stream.id, stream);

        return stream;
    }
}

/** Read a request head from a SYN's payload and refuse another shape. */
function readHead(payload: Uint8Array): RequestHead {
    // decode the JSON
    let value: unknown;
    try {
        value = JSON.parse(new TextDecoder().decode(payload));
    } catch (error) {
        throw new TypeError("stream head is no JSON", { cause: error });
    }

    // require a request head
    const parsed = RequestHead.safeParse(value);
    if (!parsed.success) {
        throw new TypeError("stream head is no request head", { cause: parsed.error });
    }

    return parsed.data;
}
