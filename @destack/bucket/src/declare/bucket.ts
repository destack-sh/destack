import { schema } from "@destack/schema";
import { defineKind, ModuleMetadata } from "@destack/package";
import { ResourceDeclaration } from "@destack/resource";
import type { Bucket } from "../bucket/index.ts";
import { S3Client } from "../s3/client.ts";

/** A file namespace bound to managed storage, and who writes its files. */
export const BucketSpec = schema.object({
    /** Only the package keeping the bucket writes its files, as its own principal. */
    write: schema.literal("package").exactOptional(),
});
/** The bucket resource kind: file storage. */
export const BucketKind = defineKind({ name: "bucket", provisioned: { spec: BucketSpec } });
/** A named file storage dependency. */
export type BucketDescription = schema.Infer<typeof BucketKind.description>;

/** Declare a file storage dependency. */
export function defineBucket(
    options: { readonly name: string; readonly spec?: schema.Input<typeof BucketSpec> },
    module?: ModuleMetadata,
): ResourceDeclaration<Bucket, typeof BucketSpec> {
    const owner = ModuleMetadata.require(module, "defineBucket").package;

    return new ResourceDeclaration(owner, BucketKind, {
        name: options.name,
        spec: BucketSpec.parse(options.spec ?? {}),
        connectors: { s3: S3Client.connector },
    });
}
