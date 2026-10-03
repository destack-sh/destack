import type { BucketCondition } from "../bucket/index.ts";
import type { S3Bucket } from "./bucket.ts";
import { S3Error } from "./error.ts";
import type { S3Request } from "./request.ts";
import { encodeUri } from "./uri.ts";
import { writeXml, type XmlElement } from "./xml.ts";

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
        throw new S3Error("InvalidArgument", "encoding-type must be url");
    }

    return encoding === "url" ? (value) => encodeUri(value, true) : (value) => value;
}
