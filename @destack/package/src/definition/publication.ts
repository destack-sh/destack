import { defineSchema, schema } from "@destack/schema";
import { Runtime } from "../runtime/index.ts";
import { DeclarationName } from "./package.ts";
import { PackageError } from "../error/error.ts";

/** An npm export condition, such as `default`, `node` or `browser`. */
export const ExportCondition = defineSchema(schema.string().regex(/^[a-z][a-z0-9_-]*$(?![\s\S])/u));
/** An npm export condition. */
export type ExportCondition = schema.Infer<typeof ExportCondition>;

/** The schema of a package's publication. */
const publicationSchema = defineSchema(
    schema.object({
        /** The module outputs a published build holds, by name. */
        outputs: schema.record(
            DeclarationName,
            schema.object({
                /** The runtime the output is built for. */
                runtime: Runtime,
            }),
        ),
        /** The output each npm export condition loads, in priority order with `default` last. */
        conditions: schema.record(ExportCondition, DeclarationName),
    }),
);
/** How the registry publishes a package: its outputs and the export condition loading each. */
export type Publication = schema.Infer<typeof publicationSchema>;

/** A package's publication. */
export const Publication = {
    /** The schema of a publication. */
    schema: publicationSchema,
    /** Require the default condition last and every condition to load a published output. */
    require,
};

/** Require the default condition last and every condition to load a published output. */
function require(publish: Publication): void {
    // require the default condition to close the priority order
    const conditions = Object.entries(publish.conditions);
    if (conditions.at(-1)?.[0] !== "default") {
        throw new PackageError("INVALID_DEFINITION", "the default export condition must be last");
    }

    // require each condition to load a published output
    for (const [condition, output] of conditions) {
        if (!Object.hasOwn(publish.outputs, output)) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `export condition ${condition} loads undeclared output ${output}`,
            );
        }
    }
}
