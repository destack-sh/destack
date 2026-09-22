import { defineSchema, schema } from "@destack/schema";
import { DependencyPackage, Package, PackageRelease } from "../package/package.ts";
import { PackageFile } from "../file/file.ts";

/** An immutable dependency release available from a registry. */
export const DependencyRelease = defineSchema(
    schema.union([
        PackageRelease.extend({
            /** The Destack registry format. */
            kind: schema.literal("destack"),
        }),
        schema.object({
            /** The npm registry format. */
            kind: schema.literal("npm"),
            /** The resolved package and release, including npm aliases. */
            package: DependencyPackage,
            /** The registry that provides the release. */
            registry: schema.string().regex(/^https?:\/\/[^\s]+$(?![\s\S])/),
            /** The registry's Subresource Integrity expression. */
            integrity: schema.string().min(1),
        }),
    ]),
);

/** An immutable dependency release available from a registry. */
export type DependencyRelease = schema.Infer<typeof DependencyRelease>;

/** A registry release or source snapshot selected during package resolution. */
export const DependencyResolution = defineSchema(
    schema.union([
        DependencyRelease,
        schema.object({
            /** Source compiled directly from a local Destack package. */
            kind: schema.literal("source"),
            /** The source package's declared name and version. */
            package: Package,
            /** Package declarations and source files consumed by the bundler. */
            files: schema.array(PackageFile),
        }),
    ]),
);

/** A registry release or source snapshot selected during package resolution. */
export type DependencyResolution = schema.Infer<typeof DependencyResolution>;
