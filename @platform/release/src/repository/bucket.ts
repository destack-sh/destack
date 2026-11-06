import { S3Client } from "bun";
import { createHash } from "node:crypto";
import type { Environment } from "@destack/environment";
import { RepositoryConfiguration } from "./configuration.ts";

/** The Cloudflare account keeping the release buckets. */
const ACCOUNT = "27c0d00fb3a27a4ccbf46a3cceab9301";

/** The bucket keeping the feed's releases. */
const BUCKET = "destack-releases-stable";

/** Maximum duration of a streamed release upload, in milliseconds. */
const UPLOAD_TIMEOUT_MILLISECONDS = 15 * 60 * 1000;

/** Maximum duration of a read from storage, in milliseconds. */
const READ_TIMEOUT_MILLISECONDS = 15_000;

/** Bucket-scoped release transport with conditional object replacement. */
export class R2ReleaseBucket {
    /** Native S3 signer configured for one bucket. */
    readonly client: S3Client;
    /** Object prefix corresponding to the selected public feed. */
    readonly prefix: string;
    /** The host the transfers run on. */
    readonly environment: Environment<"files" | "process">;

    /** Require the selected environment's bucket-scoped S3 credentials. */
    constructor(
        configuration: RepositoryConfiguration,
        environment: Environment<"files" | "process">,
    ) {
        // select the feed's R2 bucket over an encrypted S3 endpoint
        const { variables } = environment.process;
        const account = variables["CLOUDFLARE_ACCOUNT_ID"] ?? ACCOUNT;
        const bucket = variables["DESTACK_RELEASE_BUCKET"] ?? BUCKET;
        const endpoint = new URL(
            variables["DESTACK_RELEASE_S3_URL"] ?? `https://${account}.r2.cloudflarestorage.com`,
        );
        if (!RepositoryConfiguration.isClean(endpoint) || endpoint.pathname !== "/") {
            throw new Error("release storage requires a clean HTTPS origin");
        }

        // keep publication credentials separate from Cloudflare administration tokens
        const accessKeyId = variables["CLOUDFLARE_RELEASE_ACCESS_KEY_ID"];
        const secretAccessKey = variables["CLOUDFLARE_RELEASE_SECRET_ACCESS_KEY"];
        if (
            accessKeyId === undefined ||
            accessKeyId === "" ||
            secretAccessKey === undefined ||
            secretAccessKey === ""
        ) {
            throw new Error("release bucket access key and secret access key are required");
        }
        this.client = new S3Client({
            accessKeyId,
            secretAccessKey,
            bucket,
            endpoint: endpoint.href,
            region: "auto",
        });
        this.prefix = configuration.url.pathname.slice(1);
        this.environment = environment;
    }

    /** Read directly from storage without using the public download cache. */
    async get(path: string): Promise<Response> {
        const url = this.client.presign(`${this.prefix}${path}`, { method: "GET", expiresIn: 60 });
        const { clock, network } = this.environment;

        return await network.fetch(
            new Request(url, {
                redirect: "error",
                signal: clock.timeout(READ_TIMEOUT_MILLISECONDS),
            }),
        );
    }

    /** Read the authoritative object revision before a conditional replacement. */
    async revision(path: string): Promise<string | undefined> {
        // read directly from storage, bypassing public caches
        const url = this.client.presign(`${this.prefix}${path}`, { method: "HEAD", expiresIn: 60 });
        const { clock, network } = this.environment;
        const response = await network.fetch(
            new Request(url, {
                method: "HEAD",
                redirect: "error",
                signal: clock.timeout(READ_TIMEOUT_MILLISECONDS),
            }),
        );
        if (response.status === 404) {
            return undefined;
        }
        const etag = response.headers.get("etag");
        if (!response.ok || etag === null || etag === "") {
            throw new Error(`cannot read release object revision: ${path} (${response.status})`);
        }

        return etag;
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
        const { clock, files, network } = this.environment;
        await using source = await files.open(file, { read: true });
        const headers = new Headers({
            "content-type": type,
            "content-length": String((await source.stat()).size),
            "cache-control": immutable ? "public, max-age=31536000, immutable" : "no-store",
        });
        headers.set(revision === undefined ? "if-none-match" : "if-match", revision ?? "*");

        // stream the file into the presigned upload
        const url = this.client.presign(`${this.prefix}${path}`, { method: "PUT", expiresIn: 900 });
        const response = await network.fetch(
            new Request(url, {
                method: "PUT",
                headers,
                body: source.readable(),
                redirect: "error",
                signal: clock.timeout(UPLOAD_TIMEOUT_MILLISECONDS),
            }),
        );
        await response.body?.cancel();

        // accept immutable retries only when every byte matches the existing object
        if (response.status === 412 && immutable) {
            const existing = await this.get(path);
            if (!existing.ok || existing.body === null) {
                throw new Error(`cannot read release object: ${path} (${existing.status})`);
            }
            await using local = await files.open(file, { read: true });
            const [previous, next] = await Promise.all([
                digest(existing.body),
                digest(local.readable()),
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
