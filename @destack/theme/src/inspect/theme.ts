import { Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { type Theme, ThemeDefinition } from "../theme/index.ts";

/** A theme declaration as manifests describe it. */
export const ThemeDescription = defineSchema(
    ThemeDefinition.extend({
        /** The declaring package. */
        package: Package,
    }),
);
/** A theme declaration as manifests describe it. */
export type ThemeDescription = schema.Infer<typeof ThemeDescription>;

/** Describe a theme by its definition and package, so installations resolve it at runtime. */
export function describeTheme(theme: Theme): ThemeDescription {
    return { ...theme.definition, package: theme.package };
}
