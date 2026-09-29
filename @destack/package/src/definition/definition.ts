import { defineSchema, schema } from "@destack/schema";
import { Language } from "./language.ts";
import { Target } from "./target.ts";
import { Runtime } from "../runtime/index.ts";
import { TemplateDefinition } from "../template/index.ts";
import { PackageId } from "./package.ts";
import { DeclarationConstructorMap, FunctionReference } from "./constructor.ts";
import { Publication } from "./publication.ts";

/** The declarations authored in destack.json. */
const definition = defineSchema(
    schema.object({
        /** The JSON Schema address of the release that wrote the definition. */
        $schema: schema.string().regex(/^https:\/\/destack\.app\/schemas\/[^/]+\/destack\.json$/),
        /** The immutable identity assigned when creating this package. */
        id: PackageId,
        /** The language used by the package. */
        language: Language,
        /** Source generation settings for a registry template package. */
        template: TemplateDefinition.optional(),
        /** Supported targets inherited by package exports. */
        targets: schema.array(Target).min(1).optional(),
        /** Reviewed runtime compatibility shared by all exports. */
        runtimes: schema.array(Runtime).min(1).optional(),
        /** The declaration constructors the package exports, by name. */
        declarations: DeclarationConstructorMap.optional(),
        /** The extension building the packages that use this one, such as `./build#viewBuild`. */
        build: FunctionReference.optional(),
        /** How the registry publishes the package. */
        publication: Publication.schema.optional(),
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

/** The declarations authored in destack.json. */
export const PackageDefinition = Object.assign(definition, {
    /** Parse the text of a destack.json. */
    read(text: string): PackageDefinition {
        return definition.parse(JSON.parse(text));
    },
});

/** The validated contents of destack.json. */
export type PackageDefinition = schema.Infer<typeof definition>;
