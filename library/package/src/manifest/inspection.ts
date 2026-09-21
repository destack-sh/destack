import { defineSchema, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";

/** An inspection document and schema included in a package build. */
export const PackageInspectionReference = defineSchema(
    schema.object({
        /** The name of the inspection format. */
        name: schema.string().min(1),
        /** The inspection format version. */
        version: schema.number().int().min(1),
        /** The JSON document's path in the build. */
        document: PackagePath,
        /** The JSON Schema's path in the build. */
        schema: PackagePath,
    }),
);
/** An inspection document and its schema paths. */
export type PackageInspectionReference = schema.Infer<typeof PackageInspectionReference>;
