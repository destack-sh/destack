import { PackageFile } from "@destack/package/file";
import { Digest, schema } from "@destack/schema";

/** What a cache key stores: a build's manifest, or one file a transform derived. */
const entry = schema.discriminatedUnion("kind", [
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
/** What a cache key stores: a build's manifest, or one file a transform derived. */
export const BuildCacheEntry = Object.assign(entry, {
    /** List the blobs an entry references: a manifest, or a file. */
    digests(value: BuildCacheEntry): Digest[] {
        switch (value.kind) {
            // list a whole build's manifest
            case "build":
                return [value.manifest];
            // list a transform's file
            case "file":
                return [value.file.digest];
        }
    },
});
/** What a cache key stores. */
export type BuildCacheEntry = schema.Infer<typeof entry>;
