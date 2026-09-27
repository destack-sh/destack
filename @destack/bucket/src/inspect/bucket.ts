import type { Resource } from "@destack/resource";
import { BucketDescription } from "../declare/bucket.ts";

/** Describe a declared bucket for the package manifest. */
export function describeBucket(bucket: Resource<unknown, BucketDescription>): BucketDescription {
    return BucketDescription.parse({
        name: bucket.name,
        kind: bucket.kind,
        version: bucket.version,
        spec: bucket.spec,
    });
}
