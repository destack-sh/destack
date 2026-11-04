import { defineDeclarationKind, ModuleMetadata, Package } from "@destack/package";
import { Theme, ThemeDefinition, ThemeDescription } from "../theme/index.ts";

/** The theme declaration kind, which theme defines. */
export const THEME_KIND = defineDeclarationKind("theme", ThemeDescription);

/** Declare a theme, refusing text roles that read below their APCA contrast. */
export function defineTheme(definition: ThemeDefinition, module?: ModuleMetadata): Theme {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineTheme").package);

    // validate the fields and check every text role's contrast
    const theme = new Theme(owner, ThemeDefinition.parse(definition));
    theme.requireContrast();

    return theme;
}
