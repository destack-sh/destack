import { type BucketDeclaration, BucketKind, type BucketDescription } from "../declare/bucket.ts";

/** Describe a declared bucket for the package manifest. */
export function describeBucket(
    bucket: Pick<BucketDeclaration, "name" | "kind" | "spec">,
): BucketDescription {
    return BucketKind.description.parse({
        name: bucket.name,
        kind: bucket.kind,
        spec: bucket.spec,
    });
}
