import type { Metadata } from "../_generated/rpc/call/metadata.js";
import type { Completion, Message } from "../_generated/rpc/protocol/message.js";
import type { Payload } from "../_generated/rpc/protocol/payload.js";
import type { Status } from "../_generated/rpc/protocol/status.js";
import { decodeValue, encodeValue } from "./codec.js";
import { RpcProtocolError } from "./payload.js";
import type { Method, RpcResponse } from "./types.js";

/** Host operations required by one active RPC call. */
export interface CallHost {
    /** Validate one encoded call value before reserving stream capacity. */
    validatePayload(payload: Uint8Array): void;
    /** Send one call message. */
    send(message: Message): Promise<void>;
    /** Release one consumed output item. */
    releaseOutput(call: bigint): Promise<void>;
}

/** One active typed RPC invocation. */
export class Call<Response, Input, Output> implements AsyncIterableIterator<Output> {
    readonly #host: CallHost;
    readonly #id: bigint;
    readonly #method: Method<unknown, Response, Input, Output>;
    readonly #events = new EventQueue();
    readonly #inputWindow: SendWindow;
    #completion: Completion | undefined;
    #pendingOutput: Payload | undefined;
    #isInputOpen = true;
    #isComplete = false;
    #isCanceled = false;

    /** Create one active call. */
    constructor(
        host: CallHost,
        id: bigint,
        method: Method<unknown, Response, Input, Output>,
        streamWindow: number,
    ) {
        this.#host = host;
        this.#id = id;
        this.#method = method;
        this.#inputWindow = new SendWindow(streamWindow);
    }

    /** Return this caller-scoped call identifier. */
    get id(): bigint {
        return this.#id;
    }

    /** Send one caller-to-service stream item. */
    async send(input: Input): Promise<void> {
        if (!this.#isInputOpen) {
            throw new RpcProtocolError("RPC input stream is complete");
        }
        if (this.#method.input === undefined) {
            throw new RpcProtocolError("RPC method has no input stream");
        }

        const payload = encodeValue(this.#method.input, input);
        this.#host.validatePayload(payload);
        await this.#inputWindow.acquire();
        await this.#host.send({
            kind: "item",
            item: { call: this.#id, payload: { kind: "inline", inline: payload } },
        });
    }

    /** Close the caller-to-service stream. */
    async closeInput(): Promise<void> {
        if (!this.#isInputOpen) {
            return;
        }

        this.#isInputOpen = false;
        this.#inputWindow.close();
        await this.#host.send({ kind: "close", close: { call: this.#id } });
    }

    /** Receive one service-to-caller stream item. */
    async receive(): Promise<Output | undefined> {
        if (this.#pendingOutput !== undefined) {
            const payload = this.#pendingOutput;
            this.#pendingOutput = undefined;

            return this.#decodeOutput(payload);
        }
        if (this.#isComplete) {
            return undefined;
        }

        const event = await this.#events.receive();
        if (event.kind === "output") {
            return this.#decodeOutput(event.payload);
        }
        this.#acceptTerminal(event);

        return undefined;
    }

    /** Receive the terminal response after consuming every output item. */
    async response(): Promise<RpcResponse<Response>> {
        if (this.#pendingOutput !== undefined) {
            throw new RpcProtocolError("RPC response has an unread output item");
        }
        if (this.#isComplete && this.#completion === undefined) {
            throw new RpcProtocolError("RPC call is complete");
        }

        while (this.#completion === undefined) {
            const event = await this.#events.receive();
            if (event.kind === "output") {
                this.#pendingOutput = event.payload;

                throw new RpcProtocolError("RPC response has an unread output item");
            }
            this.#acceptTerminal(event);
        }

        const completion = this.#completion;
        this.#completion = undefined;
        if (completion.kind === "status") {
            throw new RpcError(completion.status);
        }
        const payload = inlinePayload(completion.response.value);
        const value = decodeValue(this.#method.response, payload);

        return { metadata: completion.response.metadata, value };
    }

    /** Request cancellation of this call. */
    async cancel(): Promise<void> {
        if (this.#isComplete) {
            throw new RpcProtocolError("RPC call is complete");
        }
        if (this.#isCanceled) {
            return;
        }

        this.#isCanceled = true;
        this.#isInputOpen = false;
        this.#inputWindow.close();
        await this.#host.send({ kind: "cancel", cancel: { call: this.#id } });
    }

    /** Return this call as its own asynchronous output iterator. */
    [Symbol.asyncIterator](): AsyncIterableIterator<Output> {
        return this;
    }

    /** Receive the next asynchronous output item. */
    async next(): Promise<IteratorResult<Output>> {
        const output = await this.receive();

        return output === undefined ? { done: true, value: undefined } : { done: false, value: output };
    }

    /** Cancel this call when asynchronous iteration ends early. */
    async return(): Promise<IteratorResult<Output>> {
        if (!this.#isComplete) {
            await this.cancel();
        }

        return { done: true, value: undefined };
    }

    /** Route one encoded output item into this call. */
    pushOutput(payload: Payload): void {
        this.#events.send({ kind: "output", payload });
    }

    /** Route one terminal completion into this call. */
    complete(completion: Completion): void {
        this.#inputWindow.close();
        this.#events.send({ kind: "complete", completion });
    }

    /** Route peer cancellation into this call. */
    canceled(): void {
        this.#inputWindow.close();
        this.#events.send({ kind: "canceled" });
    }

    /** Route one terminal connection failure into this call. */
    fail(error: Error): void {
        this.#inputWindow.close();
        this.#events.send({ kind: "failure", error });
    }

    /** Extend the caller stream send window. */
    updateInputWindow(items: number): void {
        this.#inputWindow.update(items);
    }

    /** Record one terminal event. */
    #acceptTerminal(event: Exclude<CallEvent, { kind: "output" }>): void {
        this.#isComplete = true;
        this.#isInputOpen = false;
        this.#inputWindow.close();

        if (event.kind === "complete") {
            this.#completion = event.completion;
        } else if (event.kind === "canceled") {
            throw new RpcProtocolError("RPC call was canceled");
        } else {
            throw event.error;
        }
    }

    /** Decode one output item and restore its peer send window. */
    async #decodeOutput(payload: Payload): Promise<Output> {
        if (this.#method.output === undefined) {
            throw new RpcProtocolError("RPC method has no output stream");
        }

        const output = decodeValue(this.#method.output, inlinePayload(payload));
        await this.#host.releaseOutput(this.#id);

        return output;
    }
}

/** Error returned as one terminal RPC status. */
export class RpcError extends Error {
    /** Stable failure classification. */
    readonly code: Status["code"];
    /** Encoded application-specific failure details. */
    readonly details: Uint8Array | readonly number[];
    /** Terminal failure metadata. */
    readonly metadata: Metadata;

    /** Create one error from its wire status. */
    constructor(status: Status) {
        super(status.message);
        this.name = "RpcError";
        this.code = status.code;
        this.details = status.details;
        this.metadata = status.metadata;
    }
}

/** One event routed from a connection into a call. */
type CallEvent =
    | { readonly kind: "output"; readonly payload: Payload }
    | { readonly kind: "complete"; readonly completion: Completion }
    | { readonly kind: "canceled" }
    | { readonly kind: "failure"; readonly error: Error };

/** Minimal asynchronous FIFO for call events. */
class EventQueue {
    readonly #events: CallEvent[] = [];
    readonly #receivers: Array<(event: CallEvent) => void> = [];

    /** Send one event to its receiver or queue. */
    send(event: CallEvent): void {
        const receiver = this.#receivers.shift();

        if (receiver === undefined) {
            this.#events.push(event);
        } else {
            receiver(event);
        }
    }

    /** Receive one queued or future event. */
    receive(): Promise<CallEvent> {
        const event = this.#events.shift();
        if (event !== undefined) {
            return Promise.resolve(event);
        }

        return new Promise((resolve) => this.#receivers.push(resolve));
    }
}

/** Asynchronous stream send window. */
class SendWindow {
    readonly #receivers: Array<{ readonly resolve: () => void; readonly reject: (error: Error) => void }> = [];
    #items: number;
    #isClosed = false;

    /** Create one stream send window. */
    constructor(items: number) {
        this.#items = items;
    }

    /** Reserve one item, waiting when the window is empty. */
    acquire(): Promise<void> {
        if (this.#items > 0) {
            this.#items -= 1;

            return Promise.resolve();
        }
        if (this.#isClosed) {
            return Promise.reject(new RpcProtocolError("RPC input stream is complete"));
        }

        return new Promise((resolve, reject) => this.#receivers.push({ resolve, reject }));
    }

    /** Extend this window by a bounded number of items. */
    update(items: number): void {
        if (!Number.isInteger(items) || items <= 0) {
            throw new RpcProtocolError("RPC stream window update must be positive");
        }

        let remaining = items;
        while (remaining > 0 && this.#receivers.length > 0) {
            this.#receivers.shift()!.resolve();
            remaining -= 1;
        }
        this.#items += remaining;
        if (this.#items > 0xffffffff) {
            throw new RpcProtocolError("RPC stream window overflowed");
        }
    }

    /** Close this counter and reject every pending acquire. */
    close(): void {
        if (this.#isClosed) {
            return;
        }

        this.#isClosed = true;
        const error = new RpcProtocolError("RPC input stream is complete");
        for (const receiver of this.#receivers.splice(0)) {
            receiver.reject(error);
        }
    }
}

/** Return resolved inline payload bytes. */
function inlinePayload(payload: Payload): Uint8Array {
    if (payload.kind !== "inline") {
        throw new RpcProtocolError("RPC value contains an unresolved payload");
    }

    return new Uint8Array(payload.inline);
}
