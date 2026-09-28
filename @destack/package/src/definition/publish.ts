import { defineSchema, schema } from "@destack/schema";
import { Runtime } from "../runtime/index.ts";
import { DeclarationName } from "./package.ts";
import { Target } from "./target.ts";
import { PackageError } from "../error/error.ts";

/** An npm export condition, such as `default`, `node` or `browser`. */
export const ExportCondition = defineSchema(schema.string().regex(/^[a-z][a-z0-9_-]*$(?![\s\S])/));
/** An npm export condition. */
export type ExportCondition = schema.Infer<typeof ExportCondition>;

/** How the registry publishes the package. */
export const PublishDefinition = defineSchema(
    schema.object({
        /** The module outputs a published build holds, by name. */
        outputs: schema.record(
            DeclarationName,
            schema.object({
                /** The execution target the output is built for. */
                target: Target,
                /** The runtime the output is built for, or the target's default. */
                runtime: Runtime.optional(),
            }),
        ),
        /** The output each npm export condition loads, in priority order with `default` last. */
        conditions: schema.record(ExportCondition, DeclarationName),
    }),
);
/** How the registry publishes the package. */
export type PublishDefinition = schema.Infer<typeof PublishDefinition>;

/** Require the default condition last and every condition to load a published output. */
export function requirePublish(publish: PublishDefinition): void {
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
