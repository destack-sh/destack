import { defineSchema, schema } from "@destack/schema";
import { Language, Package } from "../definition/index.ts";
import { PackageFile, Digest } from "../file/file.ts";
import { PackageOutput } from "./output.ts";

/** A description file qualified by the package defining its format. */
export const DescriptionReference = schema.object({
    /** The exact defining package. */
    package: Package,
    /** The distributed description file. */
    file: PackageFile,
});
/** A description file qualified by its defining package. */
export type DescriptionReference = schema.Infer<typeof DescriptionReference>;

/** An inspected file description selected by output and defining package. */
export const FileDescription = DescriptionReference.extend({
    /** The description kind defined by the package. */
    kind: schema.string().min(1),
    /** Outputs using this description. */
    outputs: schema.array(schema.string().min(1)),
});
/** An inspected file description. */
export type FileDescription = schema.Infer<typeof FileDescription>;

/** A distributed file and its optional inspection descriptions. */
export const ManifestFile = PackageFile.extend({
    /** Independently readable descriptions of this file. */
    descriptions: schema.array(FileDescription).optional(),
});
/** A distributed file and its inspection references. */
export type ManifestFile = schema.Infer<typeof ManifestFile>;

/** The portable structure of a package build manifest. */
export const PackageManifest = defineSchema(
    schema.object({
        /** The manifest format version, independent of the package version. */
        formatVersion: schema.literal(1),
        /** The package being built. */
        package: Package,
        /** The package's source language. */
        language: Language,
        /** Exact dependencies used by the compiler. */
        dependencies: PackageFile,
        /** Domain collections qualified by their exact defining package. */
        descriptions: schema.record(schema.string(), DescriptionReference),
        /** Named outputs compiled from the package. */
        outputs: schema.record(schema.string().regex(/^[a-z][a-z0-9-]*$(?![\s\S])/), PackageOutput),
        /** The inventory of source, executable and asset files. */
        files: PackageFile,
        /** Source maps associated with exact generated files. */
        sourceMaps: PackageFile,
    }),
);

/** A package build description. */
export type PackageManifest = schema.Infer<typeof PackageManifest>;

/** An immutable package's retrieval endpoint and manifest digest. */
export const PackageLocation = schema.object({
    /** Digest of the exact root manifest bytes. */
    manifest: Digest,
    /** Base URL serving this package's manifest, files and archive. */
    url: schema.url(),
    /** Expiry time in Unix milliseconds; absent for retained packages. */
    expiresAt: schema.number().int().nonnegative().optional(),
});
/** An immutable package's retrieval endpoint and manifest digest. */
export type PackageLocation = schema.Infer<typeof PackageLocation>;
