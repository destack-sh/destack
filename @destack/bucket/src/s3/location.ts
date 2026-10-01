import type { S3Credentials } from "./credentials.ts";
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

/** A bucket served over S3 with path-style addressing. */
export const S3Location = { url };

/** Where a bucket is served over S3, and the credentials presigning requests for its files. */
export interface BucketEndpoint {
    /** The bucket's S3 location. */
    readonly location: S3Location;
    /** The credentials presigning transfers of the bucket. */
    readonly credentials: S3Credentials;
}

/** Address a key of the bucket, or the bucket itself without a key, below the endpoint's path. */
function url(location: S3Location, key?: string): URL {
    const base = location.endpoint.pathname.replace(/\/$/, "");
    const path = key === undefined ? "" : `/${encodeUri(key, true)}`;

    return new URL(`${base}/${encodeUri(location.bucket, false)}${path}`, location.endpoint);
}
