import type { S3Bucket } from "./bucket.ts";
import { S3Condition } from "./condition.ts";
import { CorsRule } from "./cors.ts";
import type { S3Credentials } from "./credentials.ts";
import { S3Answer } from "./error.ts";
import { S3Object } from "./object.ts";
import { type CopySource, S3Action } from "./operation.ts";
import { S3Request } from "./request.ts";
import { PAYLOAD_HASH_HEADER, type S3Authorization, S3Signature } from "./signature.ts";
import { UNSIGNED_PAYLOAD } from "@destack/identity/aws";
import type {} from "@destack/package/import-meta";
import { S3Upload } from "./upload.ts";
import { decodeUri } from "./uri.ts";
import { writeErrorXml } from "./xml.ts";
import type { Clock, Environment, Random } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";
import { BucketError, preconditionFailed } from "../error/index.ts";

/** The package serving buckets over S3, which a workload's calls to its buckets address. */
export const S3_PACKAGE = import.meta.destack.package;

/** How an S3 server authenticates requests and opens buckets. */
export interface S3ServerOptions {
    /** The region clients sign requests for. */
    region: string;
    /** Look up the credentials of an access key, or undefined when none exists. */
    credentials: (accessKeyId: string) => Promise<S3Credentials | undefined>;
    /** Open a bucket an access key can use, or undefined when it has no bucket of that name. */
    open: (bucketName: string, accessKeyId: string) => Promise<S3Bucket | undefined>;
    /** The CORS rules of every bucket. */
    cors?: CorsRule[];
    /** The clock and WebCrypto checking signed requests, and the telemetry tracing them. */
    environment: Pick<Environment, "clock" | "crypto" | "random" | "telemetry">;
}

/** Serve the S3 API path-style over buckets, answering failures with S3 error documents. */
export class S3Server {
    /** The request signature verifier. */
    readonly signature: S3Signature;
    /** The CORS rules of every bucket. */
    readonly cors: CorsRule[];
    /** Look up the credentials of an access key. */
    readonly #credentials: S3ServerOptions["credentials"];
    /** Open a bucket an access key can use. */
    readonly #open: S3ServerOptions["open"];
    /** The clock signed requests are checked against. */
    readonly #clock: Clock;
    /** The randomness of request identifiers. */
    readonly #random: Random;
    /** The spans and durations of the calls. */
    readonly #scope: TelemetryScope;

    /** Serve buckets in a region. */
    constructor(options: S3ServerOptions) {
        // retain the verifier, the CORS rules and the host's lookups
        this.signature = new S3Signature({
            region: options.region,
            crypto: options.environment.crypto,
        });
        this.cors = options.cors ?? [];
        this.#credentials = options.credentials;
        this.#open = options.open;
        this.#clock = options.environment.clock;
        this.#random = options.environment.random;
        this.#scope = TelemetryScope.of(import.meta.destack.package, options.environment.telemetry);
    }

    /** The region clients sign requests for. */
    get region(): string {
        return this.signature.region;
    }

    /** Answer one S3 request signed with SigV4. */
    fetch(request: Request): Promise<Response> {
        return this.#answer(request, () =>
            this.signature.authenticate(
                request,
                (accessKeyId) => this.#credentials(accessKeyId),
                this.#clock.now(),
            ),
        );
    }

    /** Answer one S3 request its host already authorized for an access key, without SigV4. */
    serve(request: Request, accessKeyId: string): Promise<Response> {
        return this.#answer(request, async () => ({
            accessKeyId,
            payloadHash: request.headers.get(PAYLOAD_HASH_HEADER) ?? UNSIGNED_PAYLOAD,
        }));
    }

    /** Authorize and run a request, answering failures with S3 error documents. */
    async #answer(request: Request, authorize: () => Promise<S3Authorization>): Promise<Response> {
        // read the request and run it
        const requestId = this.#random.bytes(8).toHex().toUpperCase();
        const url = new URL(request.url);
        let response: Response;
        try {
            response = await this.#serve(request, url, authorize);
        } catch (error) {
            // answer failures the request caused and rethrow host failures
            const answer = S3Answer.write(error);
            if (answer === undefined) {
                throw error;
            }
            response = failureResponse(answer, request, url.pathname, requestId);
        }

        // read the request and apply the CORS rules to every answer
        const headers = new Headers(response.headers);
        headers.set("x-amz-request-id", requestId);
        if (request.method !== "OPTIONS") {
            CorsRule.apply(this.cors, request, headers);
        }

        return new Response(response.body, { status: response.status, headers });
    }

    /** Authorize a request and run its operation. */
    async #serve(
        request: Request,
        url: URL,
        authorize: () => Promise<S3Authorization>,
    ): Promise<Response> {
        // answer CORS preflights without authentication
        if (request.method === "OPTIONS") {
            return CorsRule.preflight(this.cors, request);
        }

        // address a bucket path-style, refusing the service root
        const path = url.pathname.slice(1);
        const separator = path.indexOf("/");
        const bucketName = decodeUri(separator === -1 ? path : path.slice(0, separator));
        const key = separator === -1 ? "" : decodeUri(path.slice(separator + 1));
        if (bucketName === "") {
            throw BucketError.of("OPERATION_UNSUPPORTED", "listing buckets is not supported");
        }

        // select the action, its duration by the status it answers
        const action = S3Action.select(request, url, key);
        const attributes = S3Action.attributes(action);
        const address = {
            "aws.s3.bucket": bucketName,
            ...(key === "" ? {} : { "aws.s3.key": key }),
        };
        let status: number | undefined;
        const answered = () =>
            status === undefined
                ? attributes
                : { ...attributes, "http.response.status_code": status };

        return await this.#scope.span(
            `S3/${action}`,
            { ...attributes, ...address },
            async (span) => {
                // authorize the request, then open the bucket its access key can use
                const call = new S3Request(request, bucketName, key, await authorize());
                const bucket = await this.#bucket(call, bucketName);

                // run the action, noting the status it answers
                const response = await this.#run(action, call, bucket);
                status = response.status;
                span.setAttribute("http.response.status_code", status);

                return response;
            },
            {
                kind: "server",
                duration: { name: "rpc.server.call.duration", attributes: answered },
            },
        );
    }

    /** Run an action on a bucket or one of its objects. */
    async #run(
        action: ReturnType<typeof S3Action.select>,
        call: S3Request,
        bucket: S3Bucket,
    ): Promise<Response> {
        switch (action) {
            // confirm the bucket exists
            case "HeadBucket":
                call.checkParameters([]);

                return new Response(null, { headers: { "x-amz-bucket-region": this.region } });
            case "ListObjectsV2":
                return await S3Object.list(call, bucket);
            case "ListMultipartUploads":
                return await S3Upload.list(call, bucket);
            case "DeleteObjects":
                return await S3Object.deleteMany(call, bucket);
            case "HeadObject":
                return await S3Object.head(call, bucket);
            case "GetObject":
                return await S3Object.get(call, bucket);
            case "PutObject":
                return await S3Object.put(call, bucket);
            case "CopyObject":
                return await S3Object.copy(call, bucket, await this.#source(call, bucket));
            case "DeleteObject":
                return await S3Object.delete(call, bucket);
            case "CreateMultipartUpload":
                return await S3Upload.create(call, bucket);
            case "UploadPart":
                return await S3Upload.part(call, bucket);
            case "UploadPartCopy":
                return await S3Upload.partCopy(call, bucket, await this.#source(call, bucket));
            case "ListParts":
                return await S3Upload.listParts(call, bucket);
            case "CompleteMultipartUpload":
                return await S3Upload.complete(call, bucket);
            case "AbortMultipartUpload":
                return await S3Upload.abort(call, bucket);
            // refuse other bucket operations and object methods
            case "_OTHER":
                throw call.key === ""
                    ? BucketError.of(
                          "OPERATION_UNSUPPORTED",
                          `the bucket operation '${call.request.method}' is not supported`,
                      )
                    : BucketError.of(
                          "METHOD_UNSUPPORTED",
                          `the method '${call.request.method}' on objects is not supported`,
                      );
        }
    }

    /** Open a bucket the request's access key can use. */
    async #bucket(call: S3Request, bucketName: string): Promise<S3Bucket> {
        const bucket = await this.#open(bucketName, call.authorization.accessKeyId);
        if (bucket === undefined) {
            throw BucketError.of("BUCKET_NOT_FOUND", `bucket '${bucketName}' not found`);
        }

        return bucket;
    }

    /** Open the bucket of a copy source and resolve its conditions. */
    async #source(call: S3Request, bucket: S3Bucket): Promise<CopySource> {
        // open the source bucket
        const source = call.copySource();
        const sourceBucket =
            source.bucketName === call.bucketName
                ? bucket
                : await this.#bucket(call, source.bucketName);

        // resolve the source conditions, refusing a missing source or one they already fail
        const resolution = await S3Condition.resolve(call.condition("x-amz-copy-source-"), () =>
            sourceBucket.head(source.key),
        );
        if ("failed" in resolution && resolution.failed === null) {
            throw BucketError.of("KEY_NOT_FOUND", "file key not found");
        } else if ("failed" in resolution) {
            throw preconditionFailed(resolution.failed);
        }

        return {
            bucket: sourceBucket,
            key: source.key,
            isSameBucket: sourceBucket === bucket,
            ...(resolution.onlyIf === undefined ? {} : { onlyIf: resolution.onlyIf }),
        };
    }
}

/** Answer a failure with an S3 error document, or with only its status for HEAD requests. */
function failureResponse(
    failure: S3Answer,
    request: Request,
    resource: string,
    requestId: string,
): Response {
    if (request.method === "HEAD") {
        return new Response(null, { status: failure.status });
    }
    const document = writeErrorXml({
        Code: failure.code,
        Message: failure.message,
        Resource: resource,
        RequestId: requestId,
    });

    return new Response(document, {
        status: failure.status,
        headers: { "content-type": "application/xml" },
    });
}
