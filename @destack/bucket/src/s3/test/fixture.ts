import { join } from "node:path";
import { schema } from "@destack/schema";
import { Clock, type Environment, type HostCapability } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";
import { Buffer } from "node:buffer";
import { Readable } from "node:stream";
import { S3Client } from "@aws-sdk/client-s3";
import { LocalBucket } from "../../bun/index.ts";
import { S3Server } from "../server.ts";

/** The credentials the test server accepts. */
export const CREDENTIALS = { accessKeyId: "AKIDLOCAL", secretAccessKey: "local-secret" };
/** The region the test server signs for. */
export const REGION = "us-east-1";

/** Two local buckets served over S3, and clients calling them in-process. */
export class S3Fixture implements AsyncDisposable {
    /** The served buckets by name. */
    readonly buckets: Map<string, LocalBucket>;
    /** The S3 server over the buckets. */
    readonly server: S3Server;
    /** The payload hashes of the requests SDK clients sent, in order. */
    readonly payloadHashes: string[] = [];
    /** The world the buckets live in, removed with their files on dispose. */
    readonly #simulation: Simulation;

    /** The environment the server checks signatures and presigned expiries on. */
    get environment(): Environment<HostCapability> {
        return this.#simulation.environment("host");
    }

    /** Serve the buckets, checking signed requests against the world's clock. */
    private constructor(simulation: Simulation, buckets: Map<string, LocalBucket>) {
        // keep the world and the buckets, and serve them on the world's clock
        this.#simulation = simulation;
        this.buckets = buckets;
        const environment = simulation.environment("host");
        this.server = new S3Server({
            region: REGION,
            environment,
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

    /** Open the files and archive buckets in a world at the host's time, shifted by an offset. */
    static async open(offset = 0): Promise<S3Fixture> {
        // start the world at the host's time, disposing it and every opened bucket if an open fails
        await using opening = new AsyncDisposableStack();
        const simulation = opening.use(
            new Simulation({ seed: 1, start: Clock.system.now() + offset }),
        );
        const environment = simulation.environment("host");
        const directory = await environment.files.makeTemporaryDirectory("s3-");

        // open the files and archive buckets in the world's files
        const open = async (name: string) =>
            opening.use(await LocalBucket.open(join(directory, name), "space-test", environment));
        const buckets = new Map([
            ["files", await open("files")],
            ["archive", await open("archive")],
        ]);
        opening.move();

        return new S3Fixture(simulation, buckets);
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
        const payloadHash = request.headers["x-amz-content-sha256"];
        if (payloadHash === undefined) {
            throw new TypeError("the sdk sent no payload hash");
        }
        this.payloadHashes.push(payloadHash);
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
        const body = request.body instanceof Readable ? await read(request.body) : request.body;

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
                body: Readable.from([new Uint8Array(await response.arrayBuffer())]),
            },
        };
    }

    /** Close the buckets and remove their directory. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const bucket of this.buckets.values()) {
            await bucket[Symbol.asyncDispose]();
        }
        await this.#simulation[Symbol.asyncDispose]();
    }
}

/** Read a Node stream's bytes. */
async function read(stream: Readable): Promise<Uint8Array<ArrayBuffer>> {
    const chunks: Uint8Array[] = [];
    for await (const chunk of stream) {
        if (!(chunk instanceof Uint8Array)) {
            throw new TypeError("the sdk sent a body chunk that is not bytes");
        }
        chunks.push(chunk);
    }

    return new Uint8Array(Buffer.concat(chunks));
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

/** A failed SDK call's error, read for its name and HTTP status. */
const SdkFailure = schema.looseObject({
    name: schema.string(),
    $metadata: schema.looseObject({ httpStatusCode: schema.number().exactOptional() }),
});

/** Read a failed SDK call's error name and HTTP status. */
export async function failure(call: Promise<unknown>): Promise<{ name: string; status?: number }> {
    try {
        await call;
    } catch (error) {
        const { name, $metadata } = SdkFailure.parse(error);
        const status = $metadata.httpStatusCode;

        return { name, ...(status === undefined ? {} : { status }) };
    }
    throw new Error("the call succeeded");
}
