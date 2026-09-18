import { defineSchema, schema } from "@destack/schema";
import { Language } from "./language.ts";
import { Target } from "./target.ts";
import { PackagePath } from "../file/file.ts";

/** The declarations authored in destack.json. */
export const PackageDefinition = defineSchema(schema.object({
    /** The language used by the package. */
    language: Language,
    /** The module and named export that inspect this package during a build. */
    inspect: schema.object({
        /** The package-relative inspection module. */
        module: PackagePath,
        /** The inspection export name. */
        export: schema.string().min(1),
    }).optional(),
    /** Supported targets inherited by package exports. */
    targets: schema.array(Target).min(1).optional(),
    /** Target overrides keyed by the names in package.json exports. */
    exports: schema.record(
        schema.string(),
        schema.object({
            /** Supported targets replacing the package declaration for this export. */
            targets: schema.array(Target).min(1),
        }),
    ).optional(),
}));

/** The validated contents of destack.json. */
export type PackageDefinition = schema.Infer<typeof PackageDefinition>;
