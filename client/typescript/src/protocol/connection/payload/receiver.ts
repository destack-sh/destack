import type { BinaryPayload, PayloadChunkNotification } from "../../../_generated/protocol/payload.js";
import type { WorkspaceResponse } from "../../../_generated/protocol/response.js";
import { checkedNumber } from "../integer.js";

/** Deferred payload chunks received while waiting for a response. */
export class PayloadReceiver {
    readonly #pending = new Map<number, PayloadStream>();
    readonly #completed = new Map<number, Uint8Array>();

    /** Ingest one payload chunk. */
    ingest(chunk: PayloadChunkNotification): void {
        const id = checkedNumber(chunk.id, "payload id");
        const total = chunk.total;
        const index = chunk.index;

        if (!Number.isInteger(total) || total <= 0) {
            throw new Error("payload chunk total must be positive");
        }
        if (!Number.isInteger(index) || index < 0 || index >= total) {
            throw new Error("payload chunk index out of range");
        }
        if (chunk.done && index + 1 !== total) {
            throw new Error("payload chunk done marker is inconsistent");
        }

        const stream = this.#pending.get(id) ?? new PayloadStream(total);
        this.#pending.set(id, stream);
        stream.insert(index, chunk.bytes);

        if (stream.isComplete()) {
            this.#pending.delete(id);
            this.#completed.set(id, stream.bytes());
        }
    }

    /** Resolve deferred payloads referenced by one response. */
    resolve(response: WorkspaceResponse): WorkspaceResponse | undefined {
        if (response.kind !== "runQuery") {
            return response;
        }

        const payload = this.#resolvePayload(response.run_query.payload);
        if (payload === undefined) {
            return undefined;
        }

        return {
            ...response,
            run_query: { payload },
        };
    }

    #resolvePayload(payload: BinaryPayload): BinaryPayload | undefined {
        if (payload.body.kind === "inline") {
            return payload;
        }

        const id = checkedNumber(payload.body.id, "payload id");
        const bytes = this.#completed.get(id);
        if (bytes === undefined) {
            return undefined;
        }
        if (BigInt(bytes.length) !== payload.body.totalBytes) {
            throw new Error("payload size mismatch");
        }

        this.#completed.delete(id);

        return {
            body: {
                kind: "inline",
                bytes,
            },
        };
    }
}

/** Pending payload stream. */
class PayloadStream {
    readonly #chunks: Array<Uint8Array | undefined>;
    #received = 0;

    /** Create a payload stream with a fixed chunk count. */
    constructor(total: number) {
        this.#chunks = new Array(total);
    }

    /** Insert one chunk. */
    insert(index: number, bytes: Uint8Array | readonly number[]): void {
        if (this.#chunks[index] !== undefined) {
            throw new Error("duplicate payload chunk");
        }

        this.#chunks[index] = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
        this.#received += 1;
    }

    /** Return whether all chunks have arrived. */
    isComplete(): boolean {
        return this.#received === this.#chunks.length;
    }

    /** Merge chunks into one byte buffer. */
    bytes(): Uint8Array {
        let length = 0;
        for (const chunk of this.#chunks) {
            if (chunk === undefined) {
                throw new Error("payload chunks missing at finalize");
            }

            length += chunk.length;
        }

        const bytes = new Uint8Array(length);
        let offset = 0;
        for (const chunk of this.#chunks) {
            if (chunk === undefined) {
                throw new Error("payload chunks missing at finalize");
            }

            bytes.set(chunk, offset);
            offset += chunk.length;
        }

        return bytes;
    }
}
