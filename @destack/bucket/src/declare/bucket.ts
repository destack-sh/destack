import { defineSchema, schema } from "@destack/schema";
import { ModuleMetadata } from "@destack/package";
import { defineResourceKind, ResourceDeclaration } from "@destack/resource";
import type { Bucket } from "../bucket/index.ts";

/** A file namespace bound to managed storage, and who writes its files. */
export const BucketSpec = defineSchema(
    schema.object({
        /** The system alone writes the bucket's files, absent for every caller with the write permission. */
        write: schema.literal("system").exactOptional(),
    }),
);
/** The bucket resource kind: file storage. */
export const BucketKind = defineResourceKind("bucket", { spec: BucketSpec });
/** A named file storage dependency. */
export type BucketDescription = schema.Infer<typeof BucketKind.description>;

/** Declare a file storage dependency. */
export function defineBucket(
    declaration: Omit<BucketDescription, "kind">,
    module?: ModuleMetadata,
): ResourceDeclaration<Bucket, BucketDescription> {
    const owner = ModuleMetadata.require(module, "defineBucket").package;
    const description = BucketKind.description.parse({ ...declaration, kind: "bucket" });

    return new ResourceDeclaration<Bucket, BucketDescription>(owner, description);
}
