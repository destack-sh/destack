import { encodeUri } from "./uri.ts";

/** A bucket served over S3 with path-style addressing. */
export interface S3Location {
    /** The origin serving the S3 API. */
    endpoint: URL;
    /** The bucket name. */
    bucket: string;
    /** The region requests are signed for. */
    region: string;
}

/** A bucket served over S3 with path-style addressing. */
export const S3Location = { url };

/** Address a key of the bucket, or the bucket itself without a key. */
function url(location: S3Location, key?: string): URL {
    const path = key === undefined ? "" : `/${encodeUri(key, true)}`;

    return new URL(`/${encodeUri(location.bucket, false)}${path}`, location.endpoint);
}
