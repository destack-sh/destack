import { decodePayloadFrame, encodePayloadFrame } from "../codec.js";
import { TransportError, type Transport } from "./transport.js";

/** Server that can dispatch one workspace protocol payload. */
export interface EmbeddedServer {
  /** Dispatch one encoded protocol message payload. */
  dispatch(bytes: Uint8Array): readonly unknown[];
}

/** Embedded transport backed by an in-process workspace server. */
export class EmbeddedTransport implements Transport {
  readonly #server: EmbeddedServer;
  readonly #queue: Uint8Array[] = [];
  readonly #waiters: {
    readonly resolve: (bytes: Uint8Array) => void;
    readonly reject: (error: TransportError) => void;
  }[] = [];
  #closed: TransportError | undefined;

  /** Create an embedded transport over one in-process server. */
  constructor(server: EmbeddedServer) {
    this.#server = server;
  }

  /** Send one binary frame. */
  async send(bytes: Uint8Array): Promise<void> {
    if (this.#closed !== undefined) {
      throw this.#closed;
    }

    const payload = decodePayloadFrame(bytes);
    const responses = this.#server.dispatch(payload);
    for (const response of responses) {
      this.#enqueue(encodePayloadFrame(this.#frameBytes(response)));
    }
  }

  /** Receive one binary frame. */
  receive(): Promise<Uint8Array> {
    if (this.#queue.length > 0) {
      return Promise.resolve(this.#queue.shift()!);
    }

    if (this.#closed !== undefined) {
      return Promise.reject(this.#closed);
    }

    return new Promise((resolve, reject) => {
      this.#waiters.push({ resolve, reject });
    });
  }

  /** Close the transport. */
  close(): void {
    if (this.#closed !== undefined) {
      return;
    }

    this.#closed = new TransportError("embedded transport closed");
    this.#queue.length = 0;
    while (this.#waiters.length > 0) {
      const waiter = this.#waiters.shift()!;
      waiter.reject(this.#closed);
    }
  }

  #enqueue(bytes: Uint8Array): void {
    const waiter = this.#waiters.shift();

    if (waiter !== undefined) {
      waiter.resolve(bytes);
    } else {
      this.#queue.push(bytes);
    }
  }

  #frameBytes(frame: unknown): Uint8Array {
    if (frame instanceof Uint8Array) {
      return frame;
    }

    if (frame instanceof ArrayBuffer) {
      return new Uint8Array(frame);
    }

    if (ArrayBuffer.isView(frame)) {
      return new Uint8Array(frame.buffer, frame.byteOffset, frame.byteLength);
    }

    if (Array.isArray(frame)) {
      return new Uint8Array(frame);
    }

    throw new TransportError("embedded transport frame was not binary");
  }
}
