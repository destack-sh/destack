import { BucketKind, type BucketDescription } from "../declare/bucket.ts";
import type { ResourceDeclaration } from "@destack/resource";

/** Describe a declared bucket for the package manifest. */
export function describeBucket(
    bucket: ResourceDeclaration<unknown, BucketDescription>,
): BucketDescription {
    return BucketKind.description.parse({
        name: bucket.name,
        kind: bucket.kind,
        spec: bucket.spec,
    });
}
