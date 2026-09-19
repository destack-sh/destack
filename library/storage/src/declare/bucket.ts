import { defineSchema, schema } from "@destack/schema";
import { defineResourceSchema } from "@destack/resource";

/** An object namespace bound to managed storage. */
export const BucketSpec = defineSchema(schema.object({}));
/** A named object storage dependency. */
export const BucketDeclaration = defineResourceSchema("bucket", 1, BucketSpec);
/** A named object storage dependency. */
export type BucketDeclaration = schema.Infer<typeof BucketDeclaration>;

/** Declare an object storage dependency. */
export function defineBucket(
    declaration: Omit<BucketDeclaration, "kind" | "version">,
): BucketDeclaration {
    return BucketDeclaration.parse({ ...declaration, kind: "bucket", version: 1 });
}
