import type { S3Bucket } from "./bucket.ts";
import { S3Condition } from "./condition.ts";
import { CorsRule } from "./cors.ts";
import type { S3Credentials } from "./credentials.ts";
import { S3Error } from "./error.ts";
import { S3Object } from "./object.ts";
import { type CopySource } from "./operation.ts";
import { S3Request } from "./request.ts";
import { S3Signature } from "./signature.ts";
import { S3Upload } from "./upload.ts";
import { decodeUri } from "./uri.ts";
import { writeErrorXml } from "./xml.ts";

/** How an S3 server authenticates requests and reaches buckets. */
export interface S3ServerOptions {
    /** The region clients sign requests for. */
    region: string;
    /** Look up the credentials of an access key, or undefined when none exists. */
    credentials: (accessKeyId: string) => Promise<S3Credentials | undefined>;
    /** Open a bucket an access key reaches, or undefined when it reaches no bucket of that name. */
    open: (bucketName: string, accessKeyId: string) => Promise<S3Bucket | undefined>;
    /** The CORS rules of every bucket. */
    cors?: CorsRule[];
}

/** Serve the S3 API path-style over buckets, answering failures with S3 error documents. */
export class S3Server {
    /** The request signature verifier. */
    readonly signature: S3Signature;
    /** The CORS rules of every bucket. */
    readonly cors: CorsRule[];
    /** Look up the credentials of an access key. */
    readonly #credentials: S3ServerOptions["credentials"];
    /** Open a bucket an access key reaches. */
    readonly #open: S3ServerOptions["open"];

    /** Serve buckets in a region. */
    constructor(options: S3ServerOptions) {
        // retain the verifier, the CORS rules and the host's lookups
        this.signature = new S3Signature({ region: options.region });
        this.cors = options.cors ?? [];
        this.#credentials = options.credentials;
        this.#open = options.open;
    }

    /** The region clients sign requests for. */
    get region(): string {
        return this.signature.region;
    }

    /** Answer one S3 request. */
    async fetch(request: Request): Promise<Response> {
        // read the request and run it
        const requestId = crypto.getRandomValues(new Uint8Array(8)).toHex().toUpperCase();
        const url = new URL(request.url);
        let response: Response;
        try {
            response = await this.#serve(request, url);
        } catch (error) {
            // answer failures the request caused and rethrow host failures
            const failure = S3Error.read(error);
            if (failure === undefined) {
                throw error;
            }
            response = failureResponse(failure, request, url.pathname, requestId);
        }

        // read the request and apply the CORS rules to every answer
        const headers = new Headers(response.headers);
        headers.set("x-amz-request-id", requestId);
        if (request.method !== "OPTIONS") {
            CorsRule.apply(this.cors, request, headers);
        }

        return new Response(response.body, { status: response.status, headers });
    }

    /** Authenticate a request and run its operation. */
    async #serve(request: Request, url: URL): Promise<Response> {
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
            throw new S3Error("NotImplemented", "listing buckets is not supported");
        }

        // verify the signature, then open the bucket the key reaches
        const authorization = await this.signature.authenticate(
            request,
            (accessKeyId) => this.#credentials(accessKeyId),
            Date.now(),
        );
        const call = new S3Request(request, bucketName, key, authorization);
        const bucket = await this.#bucket(call, bucketName);

        return key === ""
            ? await this.#serveBucket(call, bucket)
            : await this.#serveObject(call, bucket);
    }

    /** Run an operation on a bucket. */
    async #serveBucket(call: S3Request, bucket: S3Bucket): Promise<Response> {
        const { method } = call.request;
        const { query } = call;

        // list objects with version 2 of the listing API
        if (method === "GET" && query.get("list-type") === "2") {
            return await S3Object.list(call, bucket);
        }
        // list incomplete multipart uploads
        else if (method === "GET" && query.has("uploads")) {
            return await S3Upload.list(call, bucket);
        }
        // delete up to 1000 objects
        else if (method === "POST" && query.has("delete")) {
            return await S3Object.deleteMany(call, bucket);
        }
        // confirm the bucket exists
        else if (method === "HEAD") {
            call.checkParameters([]);

            return new Response(null, { headers: { "x-amz-bucket-region": this.region } });
        }
        // refuse other bucket operations
        else {
            throw new S3Error("NotImplemented", `the bucket operation ${method} is not supported`);
        }
    }

    /** Run an operation on an object. */
    async #serveObject(call: S3Request, bucket: S3Bucket): Promise<Response> {
        // select the operation by method, subresource and copy source
        const { method } = call.request;
        const { query } = call;
        const isCopy = call.headers.has("x-amz-copy-source");

        // read an object
        if (method === "GET" && !query.has("uploadId")) {
            return await S3Object.get(call, bucket);
        }
        // read an object's metadata
        else if (method === "HEAD" && !query.has("uploadId")) {
            return await S3Object.head(call, bucket);
        }
        // store an object, or copy one
        else if (method === "PUT" && !query.has("uploadId")) {
            return isCopy
                ? await S3Object.copy(call, bucket, await this.#source(call, bucket))
                : await S3Object.put(call, bucket);
        }
        // store a part, or copy one
        else if (method === "PUT") {
            return isCopy
                ? await S3Upload.partCopy(call, bucket, await this.#source(call, bucket))
                : await S3Upload.part(call, bucket);
        }
        // delete an object
        else if (method === "DELETE" && !query.has("uploadId")) {
            return await S3Object.delete(call, bucket);
        }
        // abort a multipart upload
        else if (method === "DELETE") {
            return await S3Upload.abort(call, bucket);
        }
        // start a multipart upload
        else if (method === "POST" && query.has("uploads")) {
            return await S3Upload.create(call, bucket);
        }
        // complete a multipart upload
        else if (method === "POST" && query.has("uploadId")) {
            return await S3Upload.complete(call, bucket);
        }
        // list the parts of a multipart upload
        else if (method === "GET") {
            return await S3Upload.listParts(call, bucket);
        }
        // refuse other methods
        else {
            throw new S3Error("MethodNotAllowed", `the method ${method} is not allowed on objects`);
        }
    }

    /** Open a bucket the request's access key reaches. */
    async #bucket(call: S3Request, bucketName: string): Promise<S3Bucket> {
        const bucket = await this.#open(bucketName, call.authorization.accessKeyId);
        if (bucket === undefined) {
            throw new S3Error("NoSuchBucket", "the specified bucket does not exist");
        }

        return bucket;
    }

    /** Open the bucket of a copy source and resolve its conditions, reusing the addressed bucket when they match. */
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
            throw new S3Error("NoSuchKey", "the specified key does not exist");
        } else if ("failed" in resolution) {
            throw new S3Error(
                "PreconditionFailed",
                "at least one of the preconditions you specified did not hold",
            );
        }

        return {
            bucket: sourceBucket,
            key: source.key,
            isSameBucket: sourceBucket === bucket,
            ...(resolution.onlyIf === undefined ? {} : { onlyIf: resolution.onlyIf }),
        };
    }
}

/** Answer a failure with an S3 error document, or with its status alone for HEAD requests. */
function failureResponse(
    failure: S3Error,
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
