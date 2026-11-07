import { Digest, schema } from "@destack/schema";
import { DependencyPackage, Package, PackageRelease } from "./package.ts";
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

/** An immutable dependency release: a Destack build, an npm unit's build, or a builtin. */
const dependencyRelease = schema.union([
    PackageRelease.extend({
        /** The Destack registry format. */
        kind: schema.literal("destack"),
    }),
    NpmRelease.extend({
        /** The npm registry format. */
        kind: schema.literal("npm"),
        /** The digest of its npm unit's build manifest: the release built for server runtimes. */
        manifest: Digest,
    }),
    schema.object({
        /** A builtin, which every runtime links at its platform's release. */
        kind: schema.literal("builtin"),
        /** The builtin package at the release the build compiled against. */
        package: Package,
    }),
]);

/** An immutable dependency release: a Destack build, an npm unit's build, or a builtin. */
export const DependencyRelease = Object.assign(dependencyRelease, {
    /** Report whether a release names its build's manifest: a Destack build's or an npm unit's. */
    hasManifest(release: DependencyRelease): release is ManifestRelease {
        return release.kind !== "builtin";
    },
});

/** An immutable dependency release: a Destack build, an npm unit's build, or a builtin. */
export type DependencyRelease = schema.Infer<typeof dependencyRelease>;

/** A dependency release with its build's manifest: a Destack build or an npm unit's. */
export type ManifestRelease = Exclude<DependencyRelease, { readonly kind: "builtin" }>;

/** A registry release or source snapshot selected during package resolution. */
export const DependencyResolution = schema.union([
    DependencyRelease,
    schema.object({
        /** Source compiled directly from a local Destack package. */
        kind: schema.literal("source"),
        /** The source package's declared name and version. */
        package: Package,
        /** Package declarations and source files consumed by the bundler. */
        files: schema.array(PackageFile),
    }),
]);

/** A registry release or source snapshot selected during package resolution. */
export type DependencyResolution = schema.Infer<typeof DependencyResolution>;
