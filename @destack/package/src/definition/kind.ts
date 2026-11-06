import type { schema } from "@destack/schema";
import type { Declaration } from "../graph/declaration.ts";
import type { BuildReader } from "../manifest/reader.ts";
import { DeclarationName, PackageId } from "./package.ts";
import { ModuleMetadata } from "./metadata.ts";

/** A declaration kind: the package defining it, its name there and its descriptions' schema. */
export class DeclarationKind<Description extends schema.Schema = schema.Schema> {
    /** The package defining the kind. */
    readonly packageId: PackageId;
    /** The kind's name within its package. */
    readonly name: DeclarationName;
    /** The schema of the kind's declarations' descriptions. */
    readonly description: Description;
    /** The declarations of this kind each read build declares, parsed once, by reader. */
    readonly #read = new WeakMap<BuildReader, Promise<Declaration<schema.Output<Description>>[]>>();

    /** Name a kind of a package, with its descriptions' schema. */
    constructor(packageId: string, name: string, description: Description) {
        this.packageId = PackageId.parse(packageId);
        this.name = DeclarationName.parse(name);
        this.description = description;
    }

    /** The fields a declaration of this kind carries: its kind's name and package. */
    get identity(): { readonly kind: DeclarationName; readonly kindPackageId: PackageId } {
        return { kind: this.name, kindPackageId: this.packageId };
    }

    /** Read a build's declarations of this kind, parsed by its schema once per reader. */
    read(build: BuildReader): Promise<Declaration<schema.Output<Description>>[]> {
        // reuse a reader's parse, as its build never changes
        const known = this.#read.get(build);
        if (known !== undefined) {
            return known;
        }

        // parse once, forgetting a failed parse
        const reading = build.declarations().then((declarations) => this.select(declarations));
        this.#read.set(build, reading);
        reading.catch(() => this.#read.delete(build));

        return reading;
    }

    /** Select this kind's declarations, their descriptions parsed by its schema. */
    select(declarations: readonly Declaration[]): Declaration<schema.Output<Description>>[] {
        return declarations
            .filter((declaration) => this.has(declaration))
            .map((declaration) => ({
                ...declaration,
                description: this.description.parse(declaration.description),
            }));
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

/** Define a declaration kind of the calling package by its name and descriptions' schema. */
export function defineDeclarationKind<Description extends schema.Schema>(
    name: string,
    description: Description,
    module?: ModuleMetadata,
): DeclarationKind<Description> {
    const owner = ModuleMetadata.require(module, "defineDeclarationKind").package;

    return new DeclarationKind(owner.id, name, description);
}
