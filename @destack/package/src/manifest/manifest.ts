import { Commit, defineSchema, Digest, Instant, schema } from "@destack/schema";
import { DeclarationName, Language, Package } from "../definition/index.ts";
import { PackageFile } from "../file/file.ts";
import { PackageOutput } from "./output.ts";

/** A description file qualified by the package defining its format. */
export const DescriptionReference = defineSchema(
    schema.object({
        /** The exact defining package. */
        package: Package,
        /** The distributed description file. */
        file: PackageFile,
    }),
);
/** A description file qualified by its defining package. */
export type DescriptionReference = schema.Infer<typeof DescriptionReference>;

/** The list files of a build, each read on demand and verified by its digest. */
export const ManifestLists = defineSchema(
    schema.object({
        /** The exact dependencies the compiler used. */
        dependencies: PackageFile,
        /** The source, executable and asset files. */
        files: PackageFile,
        /** The source maps of generated files. */
        sourceMaps: PackageFile,
        /** The index root of the build's graph: the graph file of each module. */
        graph: PackageFile,
    }),
);
/** The list files of a build. */
export type ManifestLists = schema.Infer<typeof ManifestLists>;

/** The portable structure of a package build manifest. */
export const PackageManifest = defineSchema(
    schema.object({
        /** The manifest format version, independent of the package version. */
        formatVersion: schema.literal(1),
        /** The package being built. */
        package: Package,
        /** The package's source language. */
        language: Language,
        /** The commit the build compiled, absent for a working tree with uncommitted changes. */
        commit: Commit.exactOptional(),
        /** Static test declarations qualified by the package defining their format. */
        tests: DescriptionReference.exactOptional(),
        /** The upgrade from the package's previous release, qualified by the package defining its format. */
        upgrade: DescriptionReference.exactOptional(),
        /** The list files listing the build's dependencies, files, source maps and graph modules. */
        lists: ManifestLists,
        /** Named outputs compiled from the package. */
        outputs: schema.record(DeclarationName, PackageOutput),
    }),
);

/** A package build description. */
export type PackageManifest = schema.Infer<typeof PackageManifest>;

/** An immutable package's retrieval endpoint and manifest digest. */
export const PackageLocation = defineSchema(
    schema.object({
        /** Digest of the exact root manifest bytes. */
        manifest: Digest,
        /** Base URL serving this package's manifest, files and archive. */
        url: schema.url(),
        /** Expiry time in Unix milliseconds; absent for retained packages. */
        expiresAt: Instant.exactOptional(),
    }),
);
/** An immutable package's retrieval endpoint and manifest digest. */
export type PackageLocation = schema.Infer<typeof PackageLocation>;
