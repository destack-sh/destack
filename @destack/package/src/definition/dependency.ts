import { Digest, schema } from "@destack/schema";
import { DependencyPackage, NpmPackage, PackageBuild, PackageVersion } from "./package.ts";
import { PackageFile } from "../file/file.ts";

/** An npm release the lockfile pins: its package, registry and integrity. */
export const NpmRelease = schema.object({
    /** The resolved package and release, including npm aliases. */
    package: DependencyPackage,
    /** The registry that provides the release. */
    registry: schema.string().regex(/^https?:\/\/[^\s]+$/u),
    /** The registry's Subresource Integrity expression. */
    integrity: schema.string().min(1),
});
/** An npm release the lockfile pins. */
export type NpmRelease = schema.Infer<typeof NpmRelease>;

/** A build a compile reads a dependency at: a Destack package's build, or an npm unit's. */
export const DependencyRelease = schema.union([
    PackageBuild.extend({
        /** The Destack registry format. */
        kind: schema.literal("destack"),
    }),
    schema.object({
        /** The npm registry format. */
        kind: schema.literal("npm"),
        /** The npm release, verified by its integrity. */
        package: NpmPackage,
        /** The digest of its npm unit's build manifest. */
        manifest: Digest,
    }),
]);

/** A build a compile reads a dependency at: a Destack package's build, or an npm unit's. */
export type DependencyRelease = schema.Infer<typeof DependencyRelease>;

/** A registry release or source snapshot selected during package resolution. */
export const DependencyResolution = schema.union([
    DependencyRelease,
    schema.object({
        /** Source compiled directly from a local Destack package. */
        kind: schema.literal("source"),
        /** The source package's declared name and version. */
        package: PackageVersion,
        /** Package declarations and source files consumed by the bundler. */
        files: schema.array(PackageFile),
    }),
]);

/** A registry release or source snapshot selected during package resolution. */
export type DependencyResolution = schema.Infer<typeof DependencyResolution>;
