import { type Theme, ThemeDescription } from "../theme/index.ts";

/** Describe a theme by its definition and package, so installations resolve it at runtime. */
export function describeTheme(theme: Theme): ThemeDescription {
    return { ...theme.definition, package: theme.package };
}
