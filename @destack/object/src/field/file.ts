import type { ResourceDeclaration, ResourceDescription } from "@destack/resource";
import { schema } from "@destack/schema";

/** The longest file key, as a bucket stores it. */
const MAX_KEY_LENGTH = 1024;

/** A file an object stores in a bucket its package declares, by its key. */
export const ObjectFile = schema.object({
    /** The file's key in the bucket. */
    key: schema.string().min(1).max(MAX_KEY_LENGTH),
});
/** A file an object stores in a bucket its package declares. */
export type ObjectFile = schema.Infer<typeof ObjectFile>;

/** The declaration of a bucket file fields store their files in. */
export type FileBucket = ResourceDeclaration<
    unknown,
    ResourceDescription & { readonly kind: "bucket" }
>;
