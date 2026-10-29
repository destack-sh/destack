import { PackageFile } from "@destack/package/file";
import { Digest, schema } from "@destack/schema";

/** What a cache key stores: a build's manifest, or one file a transform derived. */
export const CacheEntry = schema.discriminatedUnion("kind", [
    schema.object({
        /** A whole build. */
        kind: schema.literal("build"),
        /** The digest of the stored build's manifest. */
        manifest: Digest,
    }),
    schema.object({
        /** One file a transform derived, such as an image's rendition. */
        kind: schema.literal("file"),
        /** The stored file. */
        file: PackageFile,
    }),
]);
/** What a cache key stores. */
export type CacheEntry = schema.Infer<typeof CacheEntry>;
