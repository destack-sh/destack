import type { S3Credentials } from "./credentials.ts";
import { schema } from "@destack/schema";
import { encodeUri } from "./uri.ts";

/** A bucket served over S3 with path-style addressing. */
export interface S3Location {
    /** The URL serving the S3 API, whose path prefixes every bucket. */
    endpoint: URL;
    /** The bucket name. */
    bucket: string;
    /** The region requests are signed for. */
    region: string;
}

/** An S3 location as JSON writes it, its endpoint as text. */
const s3Location = schema.object({
    /** The URL serving the S3 API. */
    endpoint: schema.url(),
    /** The bucket name. */
    bucket: schema.string().min(1),
    /** The region requests are signed for. */
    region: schema.string().min(1),
});

/** A bucket served over S3 with path-style addressing. */
export const S3Location = { url, parse };

/** Where a bucket is served over S3, and the credentials signing requests for its files. */
export interface BucketEndpoint {
    /** The bucket's S3 location. */
    readonly location: S3Location;
    /** The credentials signing requests and presigning transfers, as their lease mode allows. */
    readonly credentials: S3Credentials;
}

/** Address a key of the bucket, or the bucket itself without a key, below the endpoint's path. */
function url(location: S3Location, key?: string): URL {
    const base = location.endpoint.pathname.replace(/\/$/u, "");
    const path = key === undefined ? "" : `/${encodeUri(key, true)}`;

    return new URL(`${base}/${encodeUri(location.bucket, false)}${path}`, location.endpoint);
}

/** Read an S3 location from its JSON. */
function parse(value: unknown): S3Location {
    const { endpoint, bucket, region } = s3Location.parse(value);

    return { endpoint: new URL(endpoint), bucket, region };
}
