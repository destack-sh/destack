import { BuildDescription } from "@destack/package/manifest";
import { PackageFile } from "@destack/package/file";
import { PackageOutput } from "@destack/package/manifest";
import { SourceMapReference } from "@destack/package/source";
import { Digest, schema } from "@destack/schema";

/** A requested output a build reuses instead of compiling it: its outputs, source maps and files. */
export const CachedOutput = schema.object({
    /** Each output the request compiled into, a kind's request into several, by name. */
    outputs: schema.record(
        schema.string(),
        schema.object({
            /** The output's manifest description. */
            output: PackageOutput,
            /** The packages, modules and files the output's compilation read and wrote. */
            description: BuildDescription,
        }),
    ),
    /** The source maps of the request's files. */
    sourceMaps: schema.array(SourceMapReference),
    /** The files the request's compilation wrote, by build-relative path. */
    files: schema.array(PackageFile),
});
/** A requested output a build reuses instead of compiling it. */
export type CachedOutput = schema.Infer<typeof CachedOutput>;

/** What a cache key names: a whole build's manifest, or one output. */
export const CacheEntry = schema.discriminatedUnion("kind", [
    schema.object({
        /** A whole build. */
        kind: schema.literal("build"),
        /** The digest of the stored build's manifest. */
        manifest: Digest,
    }),
    schema.object({
        /** One requested output of a build. */
        kind: schema.literal("output"),
        /** The requested output. */
        output: CachedOutput,
    }),
]);
/** What a cache key names. */
export type CacheEntry = schema.Infer<typeof CacheEntry>;
