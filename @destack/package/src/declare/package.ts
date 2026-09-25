import { declaringModule, type ModuleMetadata } from "../definition/metadata.ts";
import { Package } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import type { ResourceDeclaration, Declaration } from "./declaration.ts";

/** Declarations a package lets its installations bind, keyed by declaration name. */
export interface PackageDeclarationMap {
    /** Databases, buckets, vaults and other resources the package uses. */
    readonly resources?: Readonly<Record<string, ResourceDeclaration>>;
    /** Secrets the package reads. */
    readonly secrets?: Readonly<Record<string, Declaration>>;
}

/** A package's identity and the declarations its installations bind. */
export interface PackageHandle<
    Declarations extends PackageDeclarationMap = PackageDeclarationMap,
> extends Package {
    /** Resource declarations keyed by name. */
    readonly resources: NonNullable<Declarations["resources"]>;
    /** Secret declarations keyed by name. */
    readonly secrets: NonNullable<Declarations["secrets"]>;
}

/** Declare the package handle stacks import to install this package. */
export function definePackage<const Declarations extends PackageDeclarationMap>(
    declarations: Declarations,
    module?: ModuleMetadata,
): PackageHandle<Declarations> {
    // stamp the package supplied by the module transform
    const owner = Package.parse(declaringModule(module, "definePackage").package);
    const resources = declarations.resources ?? {};
    const secrets = declarations.secrets ?? {};

    // key each declaration by its own name, once across resources and secrets
    const names = new Set<string>();
    for (const [key, declaration] of [...Object.entries(resources), ...Object.entries(secrets)]) {
        if (key !== declaration.name) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `declaration ${declaration.name} is listed as ${key}`,
            );
        }
        if (names.has(key)) {
            throw new PackageError("INVALID_DEFINITION", `duplicate declaration: ${key}`);
        }
        names.add(key);
    }

    return Object.freeze({
        ...owner,
        resources,
        secrets,
    }) as PackageHandle<Declarations>;
}
