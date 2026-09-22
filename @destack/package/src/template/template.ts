import { defineSchema, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";
import { Package, PackageId, PackageName } from "../package/package.ts";

/** Source files and selectable dependencies used to create a package. */
export const TemplateDefinition = defineSchema(
    schema
        .object({
            /** Files or directories copied from the published source package. */
            files: schema.array(PackagePath).min(1),
            /** Dependencies that the caller must select. */
            dependencies: schema.array(PackageName).optional(),
        })
        .strict(),
);
/** Source files and selectable dependencies used to create a package. */
export type TemplateDefinition = schema.Infer<typeof TemplateDefinition>;

/** The new package name and dependency selections. */
export const TemplateParameters = defineSchema(
    schema
        .object({
            /** The fresh identity assigned to the generated package. */
            id: PackageId,
            /** The generated package name. */
            name: PackageName,
            /** Replacements keyed by the template's original dependency names. */
            dependencies: schema.record(PackageName, Package),
        })
        .strict(),
);
/** The new package name and dependency selections. */
export type TemplateParameters = schema.Infer<typeof TemplateParameters>;
