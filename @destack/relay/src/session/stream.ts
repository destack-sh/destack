import { Frame, FrameFlag, FrameType, MAX_PAYLOAD_BYTES } from "./frame.ts";
import { Head, RequestHead, ResponseHead } from "./head.ts";
import type { Session } from "./session.ts";

/** The bytes a stream sends before its receiver credits them at first: 256 KiB, about 5 MB/s at 50 ms. */
export const WINDOW_BYTES = 256 * 1024;

/** The widest a receiver grows a stream's window: 16 MiB, about 320 MB/s at 50 ms. */
export const MAX_WINDOW_BYTES = 16 * 1024 * 1024;

/** One HTTP exchange of a session: the request's bytes one way and the response's back, flow-controlled by credit as yamux streams are. */
export class Stream {
    /** The stream's identifier within its session. */
    readonly id: number;
    /** The head of the request the stream forwards. */
    readonly head: RequestHead;
    /** The bytes the peer sends. */
    readonly readable: ReadableStream<Uint8Array>;
    /** The bytes this side sends, a close sending FIN and an abort RST. */
    readonly writable: WritableStream<Uint8Array>;
    /** The head of the response the peer answers with, for a stream this side opened. */
    readonly answer: Promise<ResponseHead>;
    /** Aborts once the stream ends early, such as when the peer resets it. */
    readonly signal: AbortSignal;
    /** The session carrying the stream. */
    readonly #session: Session;
    /** The readable's controller. */
    #controller!: ReadableStreamDefaultController<Uint8Array>;
    /** Settles the answer. */
    readonly #answered = Promise.withResolvers<ResponseHead>();
    /** The bytes the peer lets this side send before crediting more. */
    #sendWindow = WINDOW_BYTES;
    /** The bytes this side lets the peer send before crediting more. */
    #window = WINDOW_BYTES;
    /** The bytes this side may still receive before crediting more. */
    #receiveWindow = WINDOW_BYTES;
    /** The bytes received that this side has not credited yet. */
    #owed = 0;
    /** The bytes received since the running window probe began. */
    #sample = 0;
    /** Whether a window probe waits for its ping's answer. */
    #isProbing = false;
    /** Wakes the write waiting for window, if any. */
    #credited: PromiseWithResolvers<void> | undefined;
    /** Whether this side sent FIN. */
    #isFinishSent = false;
    /** Whether the peer sent FIN. */
    #isFinishReceived = false;
    /** Aborts the stream's signal once it ends early. */
    readonly #ended = new AbortController();
    /** Why the stream ended early, once it did. */
    #failure: Error | undefined;

    /** Keep a stream of a session. */
    constructor(session: Session, id: number, head: RequestHead) {
        // keep the stream's identity
        this.id = id;
        this.head = head;
        this.#session = session;
        this.answer = this.#answered.promise;
        this.answer.catch(() => {});
        this.signal = this.#ended.signal;

        // credit the peer as readers take bytes, and reset the stream once they cancel
        this.readable = new ReadableStream<Uint8Array>(
            {
                start: (controller) => {
                    this.#controller = controller;
                },
                pull: () => this.#credit(),
                cancel: () => this.reset(new Error(`stream ${this.id} was cancelled`)),
            },
            new ByteLengthQueuingStrategy({ highWaterMark: 0 }),
        );

        // send within the window, FIN on close and RST on abort
        this.writable = new WritableStream<Uint8Array>({
            write: (chunk) => this.#send(chunk),
            close: () => this.#finish(),
            abort: (reason: unknown) =>
                this.reset(reason instanceof Error ? reason : new Error(String(reason))),
        });
    }

    /** Take a frame the peer sent for this stream. */
    receive(frame: Frame): void {
        // abort on the peer's reset
        if (frame.has(FrameFlag.rst)) {
            this.destroy(new Error(`stream ${this.id} was reset by the peer`));

            return;
        }

        // take the answer's head
        if (frame.type === FrameType.data && frame.has(FrameFlag.ack)) {
            const head = ResponseHead.safeParse(parseJson(frame.payload));
            if (!head.success) {
                this.reset(new Error(`stream ${this.id} was answered with no response head`));

                return;
            }
            this.#answered.resolve(head.data);
        }
        // take bytes within the window
        else if (frame.type === FrameType.data && frame.payload.length > 0) {
            if (frame.payload.length > this.#receiveWindow) {
                throw new TypeError(`stream ${this.id} received beyond its window`);
            }
            this.#receiveWindow -= frame.payload.length;
            this.#owed += frame.payload.length;
            this.#sample += frame.payload.length;
            this.#controller.enqueue(frame.payload);
            this.#credit();
            this.#probe();
        }
        // take more window and wake the write waiting for it
        else if (frame.type === FrameType.window) {
            this.#sendWindow += frame.value;
            this.#credited?.resolve();
            this.#credited = undefined;
        }

        // end the readable on the peer's FIN
        if (frame.has(FrameFlag.fin)) {
            this.#isFinishReceived = true;
            this.#controller.close();
            this.#settle();
        }
    }

    /** Answer the peer's request with a response, its head before its body. */
    async respond(response: Response): Promise<void> {
        // send the head
        const head = Head.response(response);
        const payload = new TextEncoder().encode(JSON.stringify(head));
        this.#session.send(new Frame(FrameType.data, FrameFlag.ack, this.id, 0, payload));

        // send the body, or end at once without one
        if (response.body === null || !Head.hasResponseBody(head)) {
            await this.writable.close();
        } else {
            await response.body.pipeTo(this.writable);
        }
    }

    /** Read the peer's request, its body the stream's readable, aborted once the stream ends early. */
    request(): Request {
        const hasBody = Head.hasRequestBody(this.head);

        return new Request(this.head.url, {
            method: this.head.method,
            headers: Head.headers(this.head),
            ...(hasBody ? { body: this.readable, duplex: "half" } : {}),
            signal: this.signal,
        });
    }

    /** End the stream early: tell the peer, unless it already ended, and fail both halves. */
    reset(error: Error): void {
        // tell the peer about an early end
        const isEnded = this.#isFinishSent && this.#isFinishReceived;
        if (this.#failure === undefined && !isEnded && !this.#session.isClosed) {
            this.#session.send(new Frame(FrameType.window, FrameFlag.rst, this.id));
        }

        this.destroy(error);
    }

    /** Fail both halves and the answer, and let the session forget the stream. */
    destroy(error: Error): void {
        // fail once
        if (this.#failure !== undefined) {
            return;
        }
        this.#failure = error;

        // fail the readable
        try {
            this.#controller.error(error);
        } catch {
            // the readable already closed or errored
        }

        // fail the waiting write
        this.#credited?.reject(error);
        this.#credited = undefined;

        // fail the answer and the request
        this.#answered.reject(error);
        this.#ended.abort(error);

        // let the session forget the stream
        this.#session.forget(this.id);
    }

    /** The bytes this side lets the peer send before crediting more. */
    get window(): number {
        return this.#window;
    }

    /** The bytes received that readers have not taken yet. */
    get #queued(): number {
        return -(this.#controller.desiredSize ?? 0);
    }

    /** Send bytes as the window allows, waiting for credit as it runs out. */
    async #send(chunk: Uint8Array): Promise<void> {
        let bytes = chunk;
        while (bytes.length > 0) {
            // refuse writes to an ended stream
            if (this.#failure !== undefined) {
                throw this.#failure;
            }

            // wait for credit once the window ran out
            if (this.#sendWindow === 0) {
                this.#credited = Promise.withResolvers<void>();
                await this.#credited.promise;
            }
            // send a frame within the window
            else {
                const length = Math.min(bytes.length, this.#sendWindow, MAX_PAYLOAD_BYTES);
                this.#session.send(
                    new Frame(FrameType.data, 0, this.id, 0, bytes.subarray(0, length)),
                );
                this.#sendWindow -= length;
                bytes = bytes.subarray(length);
            }
        }
    }

    /** Send FIN once every written byte left. */
    #finish(): void {
        this.#isFinishSent = true;
        this.#session.send(new Frame(FrameType.data, FrameFlag.fin, this.id));
        this.#settle();
    }

    /** Let the session forget the stream once both halves ended. */
    #settle(): void {
        if (this.#isFinishSent && this.#isFinishReceived) {
            this.#session.forget(this.id);
        }
    }

    /** Credit the peer for the bytes readers took once they reach half the window. */
    #credit(): void {
        // wait for half the window taken by readers
        const taken = this.#owed - this.#queued;
        if (taken < this.#window / 2 || this.#failure !== undefined) {
            return;
        }

        // credit the bytes taken
        this.#session.send(new Frame(FrameType.window, 0, this.id, taken));
        this.#receiveWindow += taken;
        this.#owed -= taken;
    }

    /** Count the bytes arriving within one ping's round trip, as gRPC estimates the bandwidth-delay product. */
    #probe(): void {
        // run one probe at a time, below the widest window
        if (this.#isProbing || this.#window >= MAX_WINDOW_BYTES) {
            return;
        }

        // count from the ping and grow after the answer
        this.#isProbing = true;
        this.#sample = 0;
        this.#session.ping().then(
            () => this.#grow(),
            // end the probe of a session that ended before answering
            () => (this.#isProbing = false),
        );
    }

    /** Double the window once a round trip carried two thirds of it to readers keeping up, crediting the growth. */
    #grow(): void {
        // end the probe, and keep a window the round trip did not fill or readers left waiting
        this.#isProbing = false;
        const isFilled = this.#sample >= (this.#window * 2) / 3;
        const isKeptUp = this.#queued < this.#window / 2;
        if (this.#failure !== undefined || !isFilled || !isKeptUp) {
            return;
        }

        // credit the peer the added window
        const growth = Math.min(this.#window, MAX_WINDOW_BYTES - this.#window);
        this.#window += growth;
        this.#receiveWindow += growth;
        this.#session.send(new Frame(FrameType.window, 0, this.id, growth));
    }
}

/** Read a payload as JSON, reading one of another kind as undefined. */
function parseJson(payload: Uint8Array): unknown {
    try {
        return JSON.parse(new TextDecoder().decode(payload));
    } catch {
        return undefined;
    }
}
