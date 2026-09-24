import { defineSchema, identifier, schema } from "@destack/schema";
import { Digest } from "../file/file.ts";

/** A concrete package export containing runnable code. */
export const Entrypoint = defineSchema(schema.string().regex(/^\.(?:\/[^\s*]+)?$(?![\s\S])/));

/** The immutable identity retained across package renames and releases. */
export const PackageId = identifier("package");
/** The immutable identity retained across package renames and releases. */
export type PackageId = schema.Infer<typeof PackageId>;

/** A declaration name within a package. */
export const DeclarationName = defineSchema(schema.string().regex(/^[a-z][a-z0-9-]*$(?![\s\S])/));
/** A declaration name within a package. */
export type DeclarationName = schema.Infer<typeof DeclarationName>;

/** A scoped Destack package name. */
export const PackageName = defineSchema(
    schema
        .string()
        .max(214)
        .regex(/^@[a-z0-9][a-z0-9._-]*\/[a-z0-9][a-z0-9._-]*$(?![\s\S])/),
);

/** A scoped or unscoped dependency name. */
export const DependencyName = defineSchema(
    schema
        .string()
        .max(214)
        .regex(/^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$(?![\s\S])/),
);

/** A named dependency version, including packages from external registries. */
export const DependencyPackage = defineSchema(
    schema.object({ name: DependencyName, version: schema.string().min(1) }),
);
/** A named dependency version. */
export type DependencyPackage = schema.Infer<typeof DependencyPackage>;

/** The immutable identity, current name and version declared by a Destack package. */
export const Package = defineSchema(
    schema.object({
        /** The identity retained across renames and releases. */
        id: PackageId,
        /** The package name, qualified by its owner. */
        name: PackageName,
        /** The package version. */
        version: schema.string().min(1),
    }),
);

/** A named, versioned package. */
export type Package = Readonly<schema.Infer<typeof Package>>;

/** An immutable Destack release and the manifest identifying its distributed contents. */
export const PackageRelease = defineSchema(
    schema.object({
        /** The released package name and version. */
        package: Package,
        /** The digest of its immutable build manifest. */
        manifest: Digest,
    }),
);
/** An immutable Destack package release. */
export type PackageRelease = schema.Infer<typeof PackageRelease>;
