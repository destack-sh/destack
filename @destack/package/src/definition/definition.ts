import { defineSchema, schema } from "@destack/schema";
import { Language } from "./language.ts";
import { Target } from "./target.ts";
import { Runtime } from "../runtime/index.ts";
import { DeclarationName } from "./package.ts";
import { TemplateDefinition } from "../template/index.ts";
import { PackageId } from "./package.ts";
import { ViewDefinition } from "../view/index.ts";

/** The declarations authored in destack.json. */
export const PackageDefinition = defineSchema(
    schema.object({
        /** The immutable identity assigned when creating this package. */
        id: PackageId,
        /** The language used by the package. */
        language: Language,
        /** Source generation settings for a registry template package. */
        template: TemplateDefinition.optional(),
        /** Named frontends opened independently by clients. */
        views: schema.record(DeclarationName, ViewDefinition).optional(),
        /** Supported targets inherited by package exports. */
        targets: schema.array(Target).min(1).optional(),
        /** Reviewed runtime compatibility shared by all exports. */
        runtimes: schema.array(Runtime).min(1).optional(),
        /** Compatibility overrides keyed by the names in package.json exports. */
        exports: schema
            .record(
                schema.string(),
                schema.object({
                    /** Supported targets replacing the package declaration for this export. */
                    targets: schema.array(Target).min(1).optional(),
                    /** Reviewed runtime compatibility replacing package defaults. */
                    runtimes: schema.array(Runtime).min(1).optional(),
                }),
            )
            .optional(),
    }),
);

/** The validated contents of destack.json. */
export type PackageDefinition = schema.Infer<typeof PackageDefinition>;
