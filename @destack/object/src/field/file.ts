import type { ResourceDeclaration, ResourceDescription } from "@destack/resource";
import { defineSchema, schema } from "@destack/schema";

/** The longest file key, as a bucket keeps it. */
const MAX_KEY_LENGTH = 1024;

/** A file an object keeps in a bucket its package declares, by its key. */
export const ObjectFile = defineSchema(
    schema.object({
        /** The file's key in the bucket. */
        key: schema.string().min(1).max(MAX_KEY_LENGTH),
    }),
);
/** A file an object keeps in a bucket its package declares. */
export type ObjectFile = schema.Infer<typeof ObjectFile>;

/** The declaration of a bucket file fields keep their files in. */
export type FileBucket = ResourceDeclaration<
    unknown,
    ResourceDescription & { readonly kind: "bucket" }
>;

/** A bucket as the manifest names it: its declaring package and its name. */
export const BucketName = Object.assign(
    defineSchema(
        schema.object({
            /** The declaring package's identifier. */
            package: schema.string().min(1),
            /** The bucket's declaration name. */
            name: schema.string().min(1),
        }),
    ),
    {
        /** Name a bucket declaration as the manifest does. */
        of(bucket: FileBucket): { readonly package: string; readonly name: string } {
            return { package: bucket.package.id, name: bucket.name };
        },
    },
);
/** A bucket as the manifest names it. */
export type BucketName = schema.Infer<typeof BucketName>;
