import type { Digest } from "@destack/schema";
import type { TransformCache } from "@destack/package/build";
import { BuildError } from "../error/index.ts";
import {
    type CacheRequest,
    type CacheResponse,
    type MessageOutput,
    writeMessage,
} from "./message.ts";

/** The build's cache as the compiler sees it, through requests its host answers. */
export class HostCache implements TransformCache {
    /** The stream the compiler's requests go out on. */
    readonly #output: MessageOutput;
    /** The requests awaiting their answer, by number. */
    readonly #pending: Map<
        number,
        {
            readonly resolve: (bytes: Uint8Array<ArrayBuffer> | null) => void;
            readonly reject: (error: unknown) => void;
        }
    >;
    /** The number of the next request. */
    #next: number;

    /** Send cache requests on the compiler's output stream. */
    constructor(output: MessageOutput) {
        this.#output = output;
        this.#pending = new Map();
        this.#next = 0;
    }

    /** Read the bytes stored under a key, absent on a miss. */
    async read(key: Digest): Promise<Uint8Array<ArrayBuffer> | undefined> {
        return (await this.#ask((id) => ({ kind: "read", id, key }))) ?? undefined;
    }

    /** Store bytes under a key for later builds. */
    async write(key: Digest, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        await this.#ask((id) => ({ kind: "write", id, key, bytes }));
    }

    /** Deliver the host's answer to the request awaiting it. */
    answer(response: CacheResponse): void {
        // find the request awaiting the answer, refusing an answer no request awaits
        const awaiting = this.#pending.get(response.id);
        if (awaiting === undefined) {
            throw BuildError.of("COMPILER_FAILED", `no pending cache request ${response.id}`);
        }

        // deliver it once
        this.#pending.delete(response.id);
        awaiting.resolve(response.bytes);
    }

    /** Fail every request awaiting an answer, as the host's input ended. */
    close(error: unknown): void {
        for (const awaiting of this.#pending.values()) {
            awaiting.reject(error);
        }
        this.#pending.clear();
    }

    /** Send a request under the next number and await its answer. */
    async #ask(request: (id: number) => CacheRequest): Promise<Uint8Array<ArrayBuffer> | null> {
        // await the answer under the next number, then send the request
        const id = this.#next++;
        const answered = new Promise<Uint8Array<ArrayBuffer> | null>((resolve, reject) => {
            this.#pending.set(id, { resolve, reject });
        });
        await writeMessage(this.#output, request(id));

        return answered;
    }
}
