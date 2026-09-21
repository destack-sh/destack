import { defineSchema, schema } from "@destack/schema";
import plugin from "../lint/index.ts";
import type { Plugin } from "../lint/plugin.ts";
import { CheckError } from "../error/index.ts";

/** A rule available to editors and package inspection. */
export const RuleDescription = defineSchema(
    schema.object({
        /** The provider-qualified rule identifier. */
        name: schema.string(),
        /** The behavior checked by this rule. */
        description: schema.string(),
        /** Whether the checker can apply a safe correction. */
        isFixable: schema.boolean(),
    }),
);

/** A rule available to editors and package inspection. */
export type RuleDescription = schema.Infer<typeof RuleDescription>;

/** Describe the custom rules supplied by this package. */
export function inspectRules(plugins: readonly Plugin[] = []): RuleDescription[] {
    // include the built-in rules and every selected dependency rule
    const providers = [{ name: plugin.meta.name, rules: plugin.rules }, ...plugins];
    const descriptions: RuleDescription[] = [];
    for (const provider of providers) {
        for (const [name, rule] of Object.entries(provider.rules)) {
            const description = rule.meta?.docs?.description;
            if (!description) {
                throw new CheckError(
                    "configuration",
                    `missing rule description: ${provider.name}/${name}`,
                );
            }
            descriptions.push({
                name: `${provider.name}/${name}`,
                description,
                isFixable: rule.meta?.fixable !== undefined,
            });
        }
    }

    return descriptions;
}
