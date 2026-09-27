import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Readable } from "node:stream";
import { S3Client } from "@aws-sdk/client-s3";
import { LocalBucket } from "../../local/index.ts";
import { S3Server } from "../server.ts";

/** The credentials the test server accepts. */
export const CREDENTIALS = { accessKeyId: "AKIDLOCAL", secretAccessKey: "local-secret" };
/** The region the test server signs for. */
export const REGION = "us-east-1";

/** Two local buckets served over S3, and clients reaching them in-process. */
export class S3Fixture implements AsyncDisposable {
    /** The served buckets by name. */
    readonly buckets: Map<string, LocalBucket>;
    /** The S3 server over the buckets. */
    readonly server: S3Server;
    /** The payload hashes of the requests SDK clients sent, in order. */
    readonly payloadHashes: string[] = [];
    /** The directory holding the buckets. */
    readonly #directory: string;

    /** Serve the buckets. */
    private constructor(directory: string, buckets: Map<string, LocalBucket>) {
        this.#directory = directory;
        this.buckets = buckets;
        this.server = new S3Server({
            region: REGION,
            credentials: async (accessKeyId) =>
                accessKeyId === CREDENTIALS.accessKeyId ? CREDENTIALS : undefined,
            open: async (name) => buckets.get(name),
            cors: [
                {
                    allowed: {
                        origins: ["https://*.example.com"],
                        methods: ["GET", "PUT"],
                        headers: ["content-type", "x-amz-*"],
                    },
                    exposeHeaders: ["etag"],
                    maxAgeSeconds: 600,
                },
            ],
        });
    }

    /** Open the files and archive buckets in a temporary directory. */
    static async open(): Promise<S3Fixture> {
        const directory = await mkdtemp(join(tmpdir(), "destack-s3-"));
        const buckets = new Map([
            ["files", await LocalBucket.open(join(directory, "files"))],
            ["archive", await LocalBucket.open(join(directory, "archive"))],
        ]);

        return new S3Fixture(directory, buckets);
    }

    /** Create an AWS SDK client that sends its requests to the server. */
    client(
        options: {
            secretAccessKey?: string;
            endpoint?: string;
            checksums?: "WHEN_SUPPORTED" | "WHEN_REQUIRED";
        } = {},
    ): S3Client {
        return new S3Client({
            region: REGION,
            endpoint: options.endpoint ?? "http://s3.test",
            forcePathStyle: true,
            maxAttempts: 1,
            credentials: { ...CREDENTIALS, ...options },
            requestChecksumCalculation: options.checksums ?? "WHEN_REQUIRED",
            responseChecksumValidation: "WHEN_REQUIRED",
            requestHandler: { handle: (request: SdkRequest) => this.#handle(request) },
        });
    }

    /** Send a request to the server as a client outside the SDK would. */
    fetch(url: string, options?: RequestInit): Promise<Response> {
        return this.server.fetch(new Request(url, options));
    }

    /** Convert an SDK request to a fetch request and its response back. */
    async #handle(request: SdkRequest): Promise<{ response: SdkResponse }> {
        // record the payload hash, then rebuild the URL and body the SDK describes
        this.payloadHashes.push(request.headers["x-amz-content-sha256"]!);
        const query = Object.entries(request.query ?? {})
            .flatMap(([name, value]) =>
                (Array.isArray(value) ? value : [value]).map((entry) =>
                    entry === null
                        ? encodeURIComponent(name)
                        : `${encodeURIComponent(name)}=${encodeURIComponent(entry)}`,
                ),
            )
            .join("&");
        const port = request.port === undefined ? "" : `:${request.port}`;
        const url = `${request.protocol}//${request.hostname}${port}${request.path}${query ? `?${query}` : ""}`;
        const body =
            request.body instanceof Readable
                ? (Readable.toWeb(request.body) as ReadableStream<Uint8Array>)
                : request.body;

        // answer with a Node stream, as the SDK reads bodies in Node runtimes
        const response = await this.server.fetch(
            new Request(url, {
                method: request.method,
                headers: request.headers,
                ...(body === undefined || body === null ? {} : { body, duplex: "half" }),
            }),
        );

        return {
            response: {
                statusCode: response.status,
                headers: Object.fromEntries(response.headers),
                body: Readable.fromWeb(
                    (response.body ??
                        new Blob().stream()) as import("node:stream/web").ReadableStream,
                ),
            },
        };
    }

    /** Close the buckets and remove their directory. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const bucket of this.buckets.values()) {
            await bucket[Symbol.asyncDispose]();
        }
        await rm(this.#directory, { recursive: true });
    }
}

/** The request shape SDK request handlers receive. */
interface SdkRequest {
    /** The HTTP method. */
    method: string;
    /** The URL scheme with its colon. */
    protocol: string;
    /** The host name. */
    hostname: string;
    /** The port, when not the scheme's default. */
    port?: number;
    /** The encoded path. */
    path: string;
    /** The decoded query parameters. */
    query?: Record<string, string | string[] | null>;
    /** The request headers. */
    headers: Record<string, string>;
    /** The request body. */
    body?: string | Uint8Array<ArrayBuffer> | ReadableStream<Uint8Array> | Readable | null;
}

/** The response shape SDK request handlers return. */
interface SdkResponse {
    /** The HTTP status. */
    statusCode: number;
    /** The response headers. */
    headers: Record<string, string>;
    /** The response body. */
    body: Readable;
}

/** Read a failed SDK call's error name and HTTP status. */
export async function failure(call: Promise<unknown>): Promise<{ name: string; status?: number }> {
    try {
        await call;
    } catch (error) {
        const { name, $metadata } = error as {
            name: string;
            $metadata: { httpStatusCode?: number };
        };

        return { name, status: $metadata.httpStatusCode };
    }
    throw new Error("the call succeeded");
}
