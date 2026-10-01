import { defineSchema, schema } from "@destack/schema";
import { declaringModule, type ModuleMetadata } from "@destack/package";
import { defineResourceKind, Resource } from "@destack/resource";
import type { Bucket } from "../bucket/index.ts";

/** A file namespace bound to managed storage. */
export const BucketSpec = defineSchema(schema.object({}));
/** The bucket resource kind: file storage. */
export const BucketKind = defineResourceKind("bucket", { spec: BucketSpec });
/** A named file storage dependency. */
export type BucketDescription = schema.Infer<typeof BucketKind.description>;

/** Declare a file storage dependency. */
export function defineBucket(
    declaration: Omit<BucketDescription, "kind">,
    module?: ModuleMetadata,
): Resource<Bucket, BucketDescription> {
    const owner = declaringModule(module, "defineBucket").package;
    const description = BucketKind.description.parse({ ...declaration, kind: "bucket" });

    return new Resource<Bucket, BucketDescription>(owner, description);
}
