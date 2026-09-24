import { S3Client } from "bun";
import { createHash } from "node:crypto";
import type { RepositoryConfiguration } from "./configuration.ts";

/** Maximum duration of a streamed release upload, in milliseconds. */
const UPLOAD_TIMEOUT_MS = 15 * 60 * 1000;

/** Bucket-scoped release transport with conditional object replacement. */
export class ReleaseBucket {
    /** Native S3 signer configured for one bucket. */
    readonly client: S3Client;
    /** Object prefix corresponding to the selected public feed. */
    readonly prefix: string;

    /** Require the selected environment's bucket-scoped S3 credentials. */
    constructor(configuration: RepositoryConfiguration) {
        // keep publication credentials separate from Cloudflare administration tokens
        const accessKeyId = process.env.CLOUDFLARE_RELEASE_ACCESS_KEY_ID;
        const secretAccessKey = process.env.CLOUDFLARE_RELEASE_SECRET_ACCESS_KEY;
        if (!accessKeyId || !secretAccessKey) {
            throw new Error("release bucket access key and secret access key are required");
        }
        this.client = new S3Client({
            accessKeyId,
            secretAccessKey,
            bucket: configuration.bucket,
            endpoint: configuration.endpoint.href,
            region: "auto",
        });
        this.prefix = configuration.url.pathname.slice(1);
    }

    /** Read directly from storage without using the public download cache. */
    async get(path: string): Promise<Response> {
        const url = this.client.presign(`${this.prefix}${path}`, { method: "GET", expiresIn: 60 });

        return await fetch(url, {
            redirect: "error",
            signal: AbortSignal.timeout(15000),
        });
    }

    /** Read the authoritative object revision before a conditional replacement. */
    async revision(path: string): Promise<string | undefined> {
        // read directly from storage, bypassing public caches
        const url = this.client.presign(`${this.prefix}${path}`, { method: "HEAD", expiresIn: 60 });
        const response = await fetch(url, {
            method: "HEAD",
            redirect: "error",
            signal: AbortSignal.timeout(15000),
        });
        if (response.status === 404) {
            return undefined;
        }
        if (!response.ok || !response.headers.get("etag")) {
            throw new Error(`cannot read release object revision: ${path} (${response.status})`);
        }

        return response.headers.get("etag")!;
    }

    /** Stream a file into a new object or replace the exact previously observed revision. */
    async put(
        path: string,
        file: string,
        type: string,
        immutable: boolean,
        revision?: string,
    ): Promise<void> {
        // apply the condition in storage so concurrent writes cannot replace observed content
        const headers = new Headers({
            "content-type": type,
            "cache-control": immutable ? "public, max-age=31536000, immutable" : "no-store",
        });
        headers.set(revision ? "if-match" : "if-none-match", revision ?? "*");
        const url = this.client.presign(`${this.prefix}${path}`, { method: "PUT", expiresIn: 900 });
        const response = await fetch(url, {
            method: "PUT",
            headers,
            body: Bun.file(file),
            redirect: "error",
            signal: AbortSignal.timeout(UPLOAD_TIMEOUT_MS),
        });
        await response.body?.cancel();

        // accept immutable retries only when every byte matches the existing object
        if (response.status === 412 && immutable) {
            const existing = this.client.file(`${this.prefix}${path}`);
            const [previous, next] = await Promise.all([
                digest(existing.stream()),
                digest(Bun.file(file).stream()),
            ]);
            if (previous !== next) {
                throw new Error(`refusing to overwrite immutable release file: ${path}`);
            }
        }
        // report object paths without exposing signed URLs
        else if (!response.ok) {
            throw new Error(`release upload failed: ${path} (${response.status})`);
        }
    }
}

/** Hash a stream with bounded memory independent of artifact size. */
async function digest(stream: ReadableStream<Uint8Array>): Promise<string> {
    // consume each chunk once without retaining previous chunks
    const hash = createHash("sha256");
    for await (const bytes of stream) {
        hash.update(bytes);
    }

    return hash.digest("hex");
}
