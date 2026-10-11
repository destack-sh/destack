import { defineSchema, schema } from "@destack/schema";
import { ModuleMetadata } from "@destack/package";
import { type Connector, defineResourceKind, ResourceDeclaration } from "@destack/resource";
import type { Bucket } from "../bucket/index.ts";
import { S3Client } from "../s3/client.ts";

/** A file namespace bound to managed storage, and who writes its files. */
export const BucketSpec = defineSchema(
    schema.object({
        /** The service alone writes the bucket's files, as its own principal, absent for every caller with the write permission. */
        write: schema.literal("service").exactOptional(),
    }),
);
/** The bucket resource kind: file storage. */
export const BucketKind = defineResourceKind("bucket", { spec: BucketSpec });
/** A named file storage dependency. */
export type BucketDescription = schema.Infer<typeof BucketKind.description>;

/** A declared bucket, which a workload reaches over S3 at its host's egress. */
export class BucketDeclaration extends ResourceDeclaration<Bucket, BucketDescription> {
    /** The connector of buckets a host serves over S3 at its egress. */
    override get connectors(): { readonly s3: Connector<Bucket> } {
        return { s3: S3Client.connector };
    }
}

/** Declare a file storage dependency. */
export function defineBucket(
    declaration: Omit<BucketDescription, "kind">,
    module?: ModuleMetadata,
): BucketDeclaration {
    const owner = ModuleMetadata.require(module, "defineBucket").package;
    const description = BucketKind.description.parse({ ...declaration, kind: "bucket" });

    return new BucketDeclaration(owner, description);
}
