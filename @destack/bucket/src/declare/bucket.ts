import { defineSchema, schema } from "@destack/schema";
import { declaringModule, type ModuleMetadata } from "@destack/package";
import { defineResourceSchema, Resource } from "@destack/resource";
import type { Bucket } from "../bucket/index.ts";

/** A file namespace bound to managed storage. */
export const BucketSpec = defineSchema(schema.object({}));
/** A named file storage dependency. */
export const BucketDescription = defineResourceSchema("bucket", 1, BucketSpec);
/** A named file storage dependency. */
export type BucketDescription = schema.Infer<typeof BucketDescription>;

/** Declare a file storage dependency. */
export function defineBucket(
    declaration: Omit<BucketDescription, "kind" | "version">,
    module?: ModuleMetadata,
): Resource<Bucket, BucketDescription> {
    const owner = declaringModule(module, "defineBucket").package;
    const description = BucketDescription.parse({ ...declaration, kind: "bucket", version: 1 });

    return new Resource<Bucket, BucketDescription>(owner, description);
}
