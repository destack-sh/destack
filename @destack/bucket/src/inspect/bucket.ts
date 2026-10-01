import type { Resource } from "@destack/resource";
import { BucketKind, type BucketDescription } from "../declare/bucket.ts";

/** Describe a declared bucket for the package manifest. */
export function describeBucket(bucket: Resource<unknown, BucketDescription>): BucketDescription {
    return BucketKind.description.parse({
        name: bucket.name,
        kind: bucket.kind,
        spec: bucket.spec,
    });
}
