import { DeclarationName, PackageId } from "./package.ts";
import { ModuleMetadata } from "./metadata.ts";

/** A declaration kind: the package defining it and its name there, such as object's `object`. */
export class DeclarationKind {
    /** The package defining the kind. */
    readonly packageId: PackageId;
    /** The kind's name within its package. */
    readonly name: DeclarationName;

    /** Name a kind of a package. */
    constructor(packageId: string, name: string) {
        this.packageId = PackageId.parse(packageId);
        this.name = DeclarationName.parse(name);
    }

    /** The fields a declaration of this kind carries: its kind's name and package. */
    get identity(): { readonly kind: DeclarationName; readonly kindPackageId: PackageId } {
        return { kind: this.name, kindPackageId: this.packageId };
    }

    /** Report whether a declaration is of this kind. */
    has(declaration: { readonly kind: string; readonly kindPackageId: string }): boolean {
        return this.is(declaration.kind, declaration.kindPackageId);
    }

    /** Report whether a kind's name and its defining package's id are this kind. */
    is(kind: string, packageId: string): boolean {
        return packageId === this.packageId && kind === this.name;
    }
}

/** Define a declaration kind of the calling package by its name, such as `command`. */
export function defineDeclarationKind(name: string, module?: ModuleMetadata): DeclarationKind {
    const owner = ModuleMetadata.require(module, "defineDeclarationKind").package;

    return new DeclarationKind(owner.id, name);
}
