import { defineSchema, schema } from "@destack/schema";
import { Language } from "./language.ts";
import { Runtime } from "../runtime/index.ts";
import { TemplateDefinition } from "../template/index.ts";
import { PackageId } from "./package.ts";
import { DeclarationConstructorMap, FunctionReference } from "./constructor.ts";
import { Publication } from "./publication.ts";
import { PackageError } from "../error/index.ts";

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
        /** The runtimes the package's exports compile for. */
        runtimes: schema.array(Runtime).min(1).optional(),
        /** The declaration constructors the package exports, by name. */
        declarations: DeclarationConstructorMap.optional(),
        /** The extension building the packages that use this one, such as `./build#viewBuild`. */
        build: FunctionReference.optional(),
        /** How the registry publishes the package. */
        publication: Publication.schema.optional(),
        /** Runtime overrides keyed by the names in package.json exports. */
        exports: schema
            .record(
                schema.string(),
                schema.object({
                    /** The runtimes this export compiles for, replacing the package's. */
                    runtimes: schema.array(Runtime).min(1),
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
    /** List the runtimes an export compiles for: its own, or else the package's. */
    runtimes(value: PackageDefinition, name: string): Runtime[] {
        const runtimes = value.exports?.[name]?.runtimes ?? value.runtimes;
        if (runtimes === undefined) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `no runtimes declared for export: ${name}`,
            );
        }

        return runtimes;
    },
});

/** The validated contents of destack.json. */
export type PackageDefinition = schema.Infer<typeof definition>;
