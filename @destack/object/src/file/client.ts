import type { Environment } from "@destack/environment";
import { Digest } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { CallError } from "../error/index.ts";
import { RetryAfter } from "../error/detail.ts";
import { FILE_PATH, type ObjectFileInput, ObjectFile, PART_BYTES, ObjectFilePart } from "./file.ts";

/** How many parts of a file are sent at once, as browsers open six connections to a host. */
const CONCURRENT_PARTS = 4;

/** How often a part is sent before its file fails. */
const PART_ATTEMPTS = 4;

/** The media type of a file whose own the sender does not know (RFC 2046 4.5.1). */
const UNKNOWN_TYPE = "application/octet-stream";

/** How long a part waits before it is sent again, doubled each time. */
const RETRY_MILLISECONDS = 500;

/** The statuses a server answers while it may take a part later. */
const RETRIED: ReadonlySet<number> = new Set([408, 425, 429, 500, 502, 503, 504]);

/** Where a store is mounted, and how requests to it are sent. */
export interface Endpoint {
    /** The URL the store is mounted at, whose query each request repeats. */
    readonly url: string;
    /** Send a request, with the caller's credential. */
    readonly fetch: (request: Request) => Promise<Response>;
}

/** The server a client sends files to, and the effects it sends them with. */
export interface ObjectFileClientOptions extends Endpoint {
    /** The clock retries wait on, and the crypto parts are hashed with. */
    readonly environment: Pick<Environment, "clock" | "crypto">;
}

/** The bytes of a file to send, as a Blob has them. */
export interface UploadSource {
    /** How many there are. */
    readonly size: number;
    /** Take the bytes from a start to an end. */
    readonly slice: (
        start: number,
        end: number,
    ) => { readonly bytes: () => Promise<Uint8Array<ArrayBuffer>> };
}

/** What a file on its way is called and what it is. */
export type UploadInput = Omit<ObjectFileInput, "parts">;

/** How far a file on its way is. */
export type UploadStatus = "sending" | "paused" | "done" | "failed" | "cancelled";

/** Why a file could not be sent: the connection was lost, or the server refused it. */
export type UploadReason = "connection" | "refusal";

/** A client's files: it sends them in parts, and writes the addresses of objects' files. */
export class ObjectFileClient {
    /** The server and effects. */
    readonly #options: ObjectFileClientOptions;

    /** Send files to a server. */
    constructor(options: ObjectFileClientOptions) {
        this.#options = options;
    }

    /** Start sending a file, of an unknown media type for an empty one. */
    upload(body: UploadSource, input: UploadInput): Upload {
        return new Upload(body, input, this.#options);
    }

    /** Open an object's file. */
    async open(
        object: ObjectReference,
        field: string,
    ): Promise<ReadableStream<Uint8Array<ArrayBuffer>>> {
        // ask for it as the caller, throwing the server's refusal
        const answer = await this.#options.fetch(new Request(this.address(object, field)));
        if (!answer.ok || answer.body === null) {
            throw await Upload.failure(answer);
        }

        return answer.body;
    }

    /** Write the address of an object's file, which a page shows it from. */
    address(object: ObjectReference, field: string): string {
        const steps = [object.scope, object.type, object.id, field];

        return addressOf(this.#options.url, `/${steps.map(encodeURIComponent).join("/")}`);
    }
}

/** Write the address of a path of the files a store mounted at a URL serves. */
function addressOf(url: string, path: string): string {
    const address = new URL(url);
    address.pathname = `${address.pathname.replace(/\/$/u, "")}${FILE_PATH}${path}`;

    return address.href;
}

/** A file on its way: its parts are sent by several at once, each again until it arrives. */
export class Upload {
    /** The file's name and media type. */
    readonly input: UploadInput;
    /** The file's size in bytes. */
    readonly size: number;
    /** The file, once the server has it. */
    readonly done: Promise<ObjectFile>;
    /** The bytes to send. */
    readonly #body: UploadSource;
    /** The server and effects. */
    readonly #options: ObjectFileClientOptions;
    /** The parts that arrived, by place. */
    readonly #parts: (ObjectFilePart | undefined)[];
    /** The places of the parts still to send. */
    readonly #waiting: number[];
    /** Aborts the requests once the upload is cancelled. */
    readonly #cancelling = new AbortController();
    /** Runs at each change. */
    readonly #listeners = new Set<() => void>();
    /** Resolves as a paused upload goes on. */
    #resumed: PromiseWithResolvers<void> | undefined;
    /** How far it is. */
    #status: UploadStatus = "sending";
    /** Whether the last request got no answer. */
    #isLost = false;

    /** Start sending a file. */
    constructor(body: UploadSource, input: UploadInput, options: ObjectFileClientOptions) {
        // cut it into parts, none for an empty file
        const count = Math.ceil(body.size / PART_BYTES);
        this.#body = body;
        this.#options = options;
        this.#parts = Array.from({ length: count }, () => undefined);
        this.#waiting = Array.from({ length: count }, (_, place) => place);
        this.input = { ...input, type: input.type === "" ? UNKNOWN_TYPE : input.type };
        this.size = body.size;

        // send it, telling whoever watches how it ended
        this.done = this.#send();
        this.done.then(
            () => this.#become("done"),
            () => this.#become(this.#cancelling.signal.aborted ? "cancelled" : "failed"),
        );
    }

    /** How far it is. */
    get status(): UploadStatus {
        return this.#status;
    }

    /** Why it failed, absent for one that did not. */
    get reason(): UploadReason | undefined {
        return this.#status === "failed" ? (this.#isLost ? "connection" : "refusal") : undefined;
    }

    /** The bytes that arrived. */
    get loaded(): number {
        return this.#parts.reduce((total, part) => total + (part?.size ?? 0), 0);
    }

    /** Send no further part until it is resumed. */
    pause(): void {
        if (this.#status === "sending") {
            this.#resumed = Promise.withResolvers<void>();
            this.#become("paused");
        }
    }

    /** Go on sending. */
    resume(): void {
        if (this.#status === "paused") {
            this.#resumed?.resolve();
            this.#resumed = undefined;
            this.#become("sending");
        }
    }

    /** Stop sending for good, which fails `done`. */
    cancel(): void {
        this.#cancelling.abort(new DOMException("the upload was cancelled", "AbortError"));
        this.#resumed?.resolve();
    }

    /** Run a listener at each change, until it stops. */
    watch(listener: () => void): () => void {
        this.#listeners.add(listener);

        return () => void this.#listeners.delete(listener);
    }

    /** Change how far it is, and tell the listeners. */
    #become(status: UploadStatus): void {
        this.#status = status;
        this.#tell();
    }

    /** Tell the listeners of a change. */
    #tell(): void {
        for (const listener of this.#listeners) {
            listener();
        }
    }

    /** Send every part, then make the file of them. */
    async #send(): Promise<ObjectFile> {
        // send several parts at once, the first failure stopping the rest
        const senders = Array.from({ length: CONCURRENT_PARTS }, () => this.#sendParts());
        try {
            await Promise.all(senders);
        } catch (error) {
            this.#waiting.length = 0;
            throw error;
        }

        // make the file
        const parts = this.#parts.filter((part) => part !== undefined);
        const answer = await this.#request("", {
            method: "POST",
            body: JSON.stringify({ ...this.input, parts } satisfies ObjectFileInput),
        });

        return ObjectFile.parse(await answer.json());
    }

    /** Send parts until none waits. */
    async #sendParts(): Promise<void> {
        for (;;) {
            // wait while it is paused, then take the next part
            await this.#resumed?.promise;
            this.#cancelling.signal.throwIfAborted();
            const place = this.#waiting.shift();
            if (place === undefined) {
                return;
            }

            // hash and send it
            const start = place * PART_BYTES;
            const bytes = await this.#body
                .slice(start, Math.min(start + PART_BYTES, this.size))
                .bytes();
            const digest = await Digest.of(bytes, this.#options.environment.crypto);
            const answer = await this.#request(`/${digest}`, { method: "PUT", body: bytes });
            this.#parts[place] = ObjectFilePart.parse(await answer.json());
            this.#tell();
        }
    }

    /** Send a request, again while the server may take it later. */
    async #request(
        path: string,
        options: {
            readonly method: string;
            readonly body: string | Uint8Array<ArrayBuffer>;
        },
    ): Promise<Response> {
        // send as the options say, until the upload is cancelled
        const { url, environment } = this.#options;
        const { clock } = environment;
        const { signal } = this.#cancelling;

        // try it, counting the attempts the server did not ask to wait for
        for (let attempt = 1; ;) {
            // send it, taking a lost connection as a failure to retry
            const request = new Request(addressOf(url, path), { ...options, signal });
            const answer = await this.#options.fetch(request).catch((error: unknown) => {
                signal.throwIfAborted();
                if (attempt === PART_ATTEMPTS) {
                    this.#isLost = true;
                    throw error;
                }
            });

            this.#isLost = answer === undefined;

            // answer a success
            if (answer?.ok === true) {
                return answer;
            }

            // refuse what no retry changes, a server over its rate saying when one does
            const due =
                answer?.status === 429 ? RetryAfter.read(answer.headers, clock.now()) : undefined;
            const isLast = attempt === PART_ATTEMPTS && due === undefined;
            if (answer !== undefined && (isLast || !RETRIED.has(answer.status))) {
                throw await Upload.failure(answer);
            }
            await answer?.body?.cancel();

            // wait as long as it asks, else longer at each attempt
            await clock.sleep(due ?? RETRY_MILLISECONDS * 2 ** (attempt - 1), signal);
            attempt += due === undefined ? 1 : 0;
        }
    }

    /** Read a refusal as the call error it is. */
    static async failure(answer: Response): Promise<Error> {
        const body: unknown = await answer.json().catch(() => undefined);

        return (
            CallError.fromJson(body) ??
            new TypeError(`upload failed: the server answered ${answer.status}`)
        );
    }
}
