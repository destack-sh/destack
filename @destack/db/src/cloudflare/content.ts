import type { ContentStore } from "@destack/resource";
import { Digest } from "@destack/schema";
import { DatabaseError } from "../error/error.ts";
import type { DurableObjectStub } from "./host.ts";

/** The path below which an object keeps content pages by digest for the operations it sends, as an OCI registry serves blobs. */
export const CONTENT_PATH = "/.destack/content";

/** The content pages another Durable Object keeps for one operation, read and written over its fetch with the operation's credential. */
export class DurableObjectContentStore implements Pick<ContentStore, "read" | "write"> {
    /** The object keeping the pages. */
    readonly #object: DurableObjectStub;
    /** The operation's credential, proving each request. */
    readonly #authorization: string;

    /** Read and write the pages an object keeps, with an operation's credential. */
    constructor(object: DurableObjectStub, authorization: string) {
        this.#object = object;
        this.#authorization = authorization;
    }

    /** Read a page's bytes. */
    async *read(digest: Digest): AsyncIterable<Uint8Array> {
        const response = await this.#send(digest, "GET");
        if (response.body === null) {
            throw new DatabaseError("CONTENT_UNAVAILABLE", `content ${digest} has no body`);
        }

        yield* response.body;
    }

    /** Keep a page under its digest and return it, refusing bytes of another digest than the expected one. */
    async write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        // gather and hash the page
        const page = await gathered(body);
        const digest = Digest.parse(await Digest.of(page));
        if (expected !== undefined && expected !== digest) {
            throw new DatabaseError("INVALID_RECORD", `content ${expected} hashes to ${digest}`);
        }

        // keep it under its digest
        await this.#send(digest, "PUT", page);

        return digest;
    }

    /** Send a request for a page, refusing an answer other than success. */
    async #send(digest: Digest, method: string, body?: Uint8Array<ArrayBuffer>): Promise<Response> {
        const response = await this.#object.fetch(
            new Request(`https://content${CONTENT_PATH}/${digest}`, {
                method,
                headers: { authorization: this.#authorization },
                ...(body === undefined ? {} : { body }),
            }),
        );
        if (!response.ok) {
            const text = await response.text();
            throw new DatabaseError(
                "CONTENT_UNAVAILABLE",
                `the object keeping content refused ${method} ${digest}: ${response.status} ${text}`,
            );
        }

        return response;
    }
}

/** Gather a body's chunks into one array of bytes. */
export async function gathered(body: AsyncIterable<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    const chunks = await Array.fromAsync(body);

    return new Blob(chunks.map((chunk) => new Uint8Array(chunk))).bytes();
}
