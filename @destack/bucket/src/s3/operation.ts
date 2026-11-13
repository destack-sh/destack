import type { Attributes } from "@destack/telemetry";
import type { BucketCondition } from "../bucket/index.ts";
import type { S3Bucket } from "./bucket.ts";
import { BucketError } from "../error/index.ts";
import type { S3Request } from "./request.ts";
import { encodeUri, readQuery } from "./uri.ts";
import { writeXml, type XmlElement } from "./xml.ts";

/** The RPC system of S3 calls, as OpenTelemetry's `rpc.system.name` reports AWS APIs. */
const RPC_SYSTEM = "aws-api";

/** The RPC service of S3 calls, as OpenTelemetry's `rpc.service` reports AWS services. */
const RPC_SERVICE = "S3";

/** The method of a request no S3 action answers, as OpenTelemetry reports unknown methods. */
const OTHER_ACTION = "_OTHER";

/** An S3 action the client calls or the server answers, as the S3 API names it. */
export type S3Action =
    | "HeadBucket"
    | "ListObjectsV2"
    | "ListMultipartUploads"
    | "DeleteObjects"
    | "HeadObject"
    | "GetObject"
    | "PutObject"
    | "CopyObject"
    | "DeleteObject"
    | "CreateMultipartUpload"
    | "UploadPart"
    | "UploadPartCopy"
    | "ListParts"
    | "CompleteMultipartUpload"
    | "AbortMultipartUpload";

/** S3 actions. */
export const S3Action = {
    /** Select the action a request asks for by its method, subresource and copy source. */
    select(request: Request, url: URL, key: string): S3Action | typeof OTHER_ACTION {
        // read the subresources and a copy source, sent as a header or a presigned parameter
        const query = new Map(readQuery(url));
        const isCopy =
            request.headers.has("x-amz-copy-source") ||
            [...query.keys()].some((name) => name.toLowerCase() === "x-amz-copy-source");
        const isUpload = query.has("uploadId");
        const isBucket = key === "";
        const method = request.method;

        // list objects with version 2 of the listing API
        if (isBucket && method === "GET" && query.get("list-type") === "2") {
            return "ListObjectsV2";
        }
        // list incomplete multipart uploads
        else if (isBucket && method === "GET" && query.has("uploads")) {
            return "ListMultipartUploads";
        }
        // delete up to 1000 objects
        else if (isBucket && method === "POST" && query.has("delete")) {
            return "DeleteObjects";
        }
        // confirm the bucket exists
        else if (isBucket && method === "HEAD") {
            return "HeadBucket";
        }
        // refuse other bucket operations
        else if (isBucket) {
            return OTHER_ACTION;
        }
        // read an object
        else if (method === "GET" && !isUpload) {
            return "GetObject";
        }
        // read an object's metadata
        else if (method === "HEAD" && !isUpload) {
            return "HeadObject";
        }
        // store an object, or copy one
        else if (method === "PUT" && !isUpload) {
            return isCopy ? "CopyObject" : "PutObject";
        }
        // store a part, or copy one
        else if (method === "PUT") {
            return isCopy ? "UploadPartCopy" : "UploadPart";
        }
        // delete an object
        else if (method === "DELETE" && !isUpload) {
            return "DeleteObject";
        }
        // abort a multipart upload
        else if (method === "DELETE") {
            return "AbortMultipartUpload";
        }
        // start a multipart upload
        else if (method === "POST" && query.has("uploads")) {
            return "CreateMultipartUpload";
        }
        // complete a multipart upload
        else if (method === "POST" && isUpload) {
            return "CompleteMultipartUpload";
        }
        // list the parts of a multipart upload
        else if (method === "GET") {
            return "ListParts";
        }
        // refuse other methods on objects
        else {
            return OTHER_ACTION;
        }
    },

    /** Read the RPC attributes of an action, as OpenTelemetry's AWS conventions report them. */
    attributes(action: S3Action | typeof OTHER_ACTION): Attributes {
        return { "rpc.system.name": RPC_SYSTEM, "rpc.service": RPC_SERVICE, "rpc.method": action };
    },
};

/** The bucket and key a copy reads, with its source conditions resolved. */
export interface CopySource {
    /** The bucket with the source. */
    bucket: S3Bucket;
    /** The source key. */
    key: string;
    /** Whether the source shares the destination's bucket. */
    isSameBucket: boolean;
    /** The source conditions the bucket evaluates atomically with the copy. */
    onlyIf?: BucketCondition;
}

/** Answer with an S3 XML document. */
export function xmlResponse(name: string, root: XmlElement): Response {
    return new Response(writeXml(name, root), { headers: { "content-type": "application/xml" } });
}

/** Select the encoder of listed keys: URL encoding when the request asks for it, else none. */
export function keyEncoder(call: S3Request): (value: string) => string {
    const encoding = call.query.get("encoding-type");
    if (encoding !== undefined && encoding !== "url") {
        throw BucketError.of("ARGUMENT_INVALID", "encoding-type is invalid: it must be url");
    }

    return encoding === "url" ? (value) => encodeUri(value, true) : (value) => value;
}
