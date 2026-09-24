import { UpdateError } from "../error/error.ts";
import { BaseFetcher } from "tuf-js";

/** Maximum duration of an archive request, in milliseconds. */
const DOWNLOAD_TIMEOUT_MS = 300_000;

/** Stream archive bytes through TUF's length and digest verification. */
export class DownloadFetcher extends BaseFetcher {
    /** Signed archive length. */
    private readonly total: number;
    /** Cancellation and progress for this download. */
    private readonly options: DownloadOptions;

    /** Configure one target download. */
    constructor(total: number, options: DownloadOptions) {
        super();
        this.total = total;
        this.options = options;
    }

    /** Fetch an archive and report received bytes before TUF verifies the complete file. */
    async fetch(url: string): Promise<ReadableStream<Uint8Array<ArrayBuffer>>> {
        // keep cancellation active until the response body has finished streaming
        const timeout = AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS);
        const signal = this.options.signal
            ? AbortSignal.any([timeout, this.options.signal])
            : timeout;
        const response = await fetch(url, { signal });
        if (!response.ok || !response.body) {
            await response.body?.cancel();
            throw new UpdateError("DOWNLOAD", `Archive request failed: HTTP ${response.status}.`);
        }

        // report the signed length rather than trusting the HTTP content length
        let received = 0;
        const { total, options } = this;

        return response.body.pipeThrough(
            new TransformStream({
                transform(chunk, controller) {
                    received += chunk.byteLength;
                    options.onProgress?.({ received, total });
                    controller.enqueue(chunk);
                },
            }),
        );
    }
}

/** Download inputs shared by CLI and native desktop callers. */
export interface DownloadOptions {
    /** Authenticate an installer archive already present on disk. */
    archive?: string;
    /** Cancel an in-progress download. */
    signal?: AbortSignal;
    /** Report received bytes; completion still requires signature and digest verification. */
    onProgress?: (progress: DownloadProgress) => void;
}

/** Byte counts for the selected archive. */
export interface DownloadProgress {
    /** Bytes received so far. */
    received: number;
    /** Expected byte count from signed metadata. */
    total: number;
}
