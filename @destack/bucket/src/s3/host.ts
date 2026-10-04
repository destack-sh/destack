import type { LeaseMode } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import type { Bucket } from "../bucket/index.ts";
import type { BucketEndpoint } from "./location.ts";

/** A bucket resource of a space. */
export interface BucketReference {
    /** The space of the bucket. */
    readonly scope: Identifier<"space">;
    /** The bucket. */
    readonly bucketId: Identifier<"bucket">;
}

/** The host keeping buckets: it opens them and locates the S3 endpoints serving them. */
export interface BucketHost {
    /** Open a bucket the host keeps. */
    open(bucket: BucketReference): Promise<Bucket>;
    /** Locate the S3 endpoint serving a bucket and the credentials presigning a transfer, refusing a write into a fenced bucket. */
    locate(bucket: BucketReference, mode: LeaseMode): Promise<BucketEndpoint>;
}
