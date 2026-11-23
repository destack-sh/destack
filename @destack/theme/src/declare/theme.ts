import { defineKind, ModuleMetadata, PackageVersion } from "@destack/package";
import { Theme, ThemeDefinition, ThemeDescription } from "../theme/index.ts";

/** The theme declaration kind, which theme defines. */
export const THEME_KIND = defineKind({ name: "theme", description: ThemeDescription });

/** A theme a package declares. */
export class ThemeDeclaration extends Theme {
    /** The declaring package. */
    readonly package: PackageVersion;

    /** Declare a theme of a package. */
    constructor(owner: PackageVersion, definition: ThemeDefinition) {
        super(definition);
        this.package = owner;
    }
}

/** Declare a theme. */
export function defineTheme(
    definition: ThemeDefinition,
    module?: ModuleMetadata,
): ThemeDeclaration {
    // stamp the declaring package on the validated fields
    const owner = PackageVersion.parse(ModuleMetadata.require(module, "defineTheme").package);

    return new ThemeDeclaration(owner, ThemeDefinition.parse(definition));
}
