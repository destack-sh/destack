import { ModuleMetadata, Package } from "@destack/package";
import { Theme, ThemeDefinition } from "../theme/index.ts";

/** Declare a theme, refusing text roles that read below their APCA contrast. */
export function defineTheme(definition: ThemeDefinition, module?: ModuleMetadata): Theme {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineTheme").package);

    // validate the fields and check every text role's contrast
    const theme = new Theme(owner, ThemeDefinition.parse(definition));
    theme.requireContrast();

    return theme;
}
