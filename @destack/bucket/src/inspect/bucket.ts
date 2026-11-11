import type { ResourceDeclaration } from "@destack/resource";
import { BucketKind, type BucketDescription, type BucketSpec } from "../declare/bucket.ts";

/** Describe a declared bucket for the package manifest. */
export function describeBucket(
    bucket: Pick<ResourceDeclaration<unknown, typeof BucketSpec>, "name" | "spec">,
): BucketDescription {
    return BucketKind.description.parse({
        name: bucket.name,
        spec: bucket.spec,
    });
}
