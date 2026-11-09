import { join } from "node:path";
import { BaseFetcher } from "tuf-js";
// NOTE #Robustness: tuf-js matches these errors by class without exporting them, so they come from its internal path
import { DownloadHTTPError, DownloadLengthMismatchError } from "tuf-js/dist/error";
import type { Environment } from "@destack/environment";

/** The longest a metadata or target request takes, in milliseconds: TUF's default 30 seconds. */
const METADATA_TIMEOUT_MILLISECONDS = 30_000;

/** A download: signed metadata, or a signed target. */
export type Download = { readonly kind: "metadata" } | { readonly kind: "target" };

/** A TUF fetcher downloading through the environment's network into its files. */
export class Fetcher extends BaseFetcher {
    /** What the fetcher downloads. */
    readonly #download: Download;
    /** The clock, network and files a download runs on. */
    readonly #environment: Environment<"files">;

    /** Download metadata or a target in an environment. */
    constructor(download: Download, environment: Environment<"files">) {
        super();
        this.#download = download;
        this.#environment = environment;
    }

    /** Write a download into a temporary file up to a length, then hand it to TUF. */
    override async downloadFile<Result>(
        url: string,
        maxLength: number,
        handler: (file: string) => Promise<Result>,
    ): Promise<Result> {
        // write the fetched bytes into a temporary file, refusing more than the signed length
        const { scheduler, files } = this.#environment;
        const directory = await files.makeTemporaryDirectory("destack-tuf-");
        try {
            const file = join(directory, "download");
            const stream = await this.fetch(url);
            await using handle = await files.open(file, { write: true, createNew: true });
            const writer = handle.writable().getWriter();
            let received = 0;
            for await (const chunk of stream) {
                received += chunk.byteLength;
                if (received > maxLength) {
                    throw new DownloadLengthMismatchError("the download exceeds its signed length");
                }
                await writer.write(chunk);
            }
            await writer.close();

            // verify it as TUF does, its own file reads tracked as a library boundary
            return await scheduler.run(() => handler(file));
        } finally {
            await files.remove(directory, { recursive: true });
        }
    }

    /** Fetch a file within its timeout. */
    async fetch(url: string): Promise<ReadableStream<Uint8Array<ArrayBuffer>>> {
        // fetch within the timeout, the deadline ending with the body
        const deadline = new Deadline(this.#environment, METADATA_TIMEOUT_MILLISECONDS, undefined);
        const response = await deadline.fetch(url);

        // refuse a failed request with its status, as TUF reads a missing next root
        if (!response.ok || !response.body) {
            await response.body?.cancel();
            deadline.end();
            throw new DownloadHTTPError(
                `${this.#download.kind} request failed: HTTP ${response.status}`,
                response.status,
            );
        }

        return deadline.watch(response.body, () => undefined);
    }
}

/** Download inputs shared by CLI and native desktop callers. */
export interface DownloadOptions {
    /** The directory with a copy of the release's files, such as the running application's. */
    source?: string;
    /** Cancel an in-progress download. */
    signal?: AbortSignal;
    /** Report received bytes, the completion still requiring signature and digest verification. */
    onProgress?: (progress: DownloadProgress) => void;
}

/** Byte counts of the selected release's files. */
export interface DownloadProgress {
    /** Bytes received so far. */
    received: number;
    /** Expected byte count from signed metadata. */
    total: number;
}

/** A request's deadline on the environment's clock, aborting it unless its body ends in time. */
class Deadline {
    /** The environment whose network the request uses and whose clock times it. */
    readonly #environment: Environment;
    /** The signal aborting the request at the deadline or on the caller's cancellation. */
    readonly #signal: AbortSignal;
    /** The disarm of the timer once the body ends. */
    readonly #disarm = new AbortController();

    /** Arm a timer of some milliseconds, beside the caller's own cancellation. */
    constructor(environment: Environment, milliseconds: number, signal: AbortSignal | undefined) {
        // abort at the deadline unless disarmed first
        this.#environment = environment;
        const expired = new AbortController();
        void environment.clock.sleep(milliseconds, this.#disarm.signal).then(
            () => expired.abort(new DOMException("signal timed out", "TimeoutError")),
            () => undefined,
        );
        this.#signal =
            signal === undefined ? expired.signal : AbortSignal.any([expired.signal, signal]);
    }

    /** Fetch a URL under the deadline, ending it when the request fails. */
    async fetch(url: string): Promise<Response> {
        try {
            return await this.#environment.network.fetch(
                new Request(url, { signal: this.#signal }),
            );
        } catch (error) {
            this.end();
            throw error;
        }
    }

    /** Pass a body through, reading each chunk, and end the timer once it ends or is cancelled. */
    watch(
        body: ReadableStream<Uint8Array<ArrayBuffer>>,
        read: (chunk: Uint8Array<ArrayBuffer>) => void,
    ): ReadableStream<Uint8Array<ArrayBuffer>> {
        const reader = body.getReader();

        return new ReadableStream<Uint8Array<ArrayBuffer>>({
            pull: async (controller) => {
                // pass the next chunk on, ending the timer with the body
                const next = await reader.read();
                if (next.done) {
                    this.end();
                    controller.close();
                } else {
                    read(next.value);
                    controller.enqueue(next.value);
                }
            },
            cancel: async (reason: unknown) => {
                // end the timer and the body together
                this.end();
                await reader.cancel(reason);
            },
        });
    }

    /** Disarm the timer. */
    end(): void {
        this.#disarm.abort();
    }
}
