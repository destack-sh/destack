import { defineSchema, schema } from "@destack/schema";
import { Language, Package } from "../package/index.ts";
import { PackageFile } from "../file/file.ts";
import { PackageOutput } from "./output.ts";
import { SourceMapReference } from "../source/map.ts";

/** The portable structure of a package build manifest. */
export const PackageManifest = defineSchema(
    schema.object({
        /** The manifest format version, independent of the package version. */
        formatVersion: schema.literal(1),
        /** The package being built. */
        package: Package,
        /** The package's source language. */
        language: Language,
        /** Named outputs compiled from the package. */
        outputs: schema.record(schema.string().regex(/^[a-z][a-z0-9-]*$(?![\s\S])/), PackageOutput),
        /** Every distributed file except this manifest. */
        files: schema.array(PackageFile),
        /** Source maps associated with exact generated files. */
        sourceMaps: schema.array(SourceMapReference),
    }),
);

/** A package build description. */
export type PackageManifest = schema.Infer<typeof PackageManifest>;
