import type { ThemeDeclaration } from "../declare/index.ts";
import type { ThemeDescription } from "../theme/index.ts";

/** Describe a theme by its definition and package, so installations resolve it at runtime. */
export function describeTheme(theme: ThemeDeclaration): ThemeDescription {
    return { ...theme.definition, package: theme.package };
}
