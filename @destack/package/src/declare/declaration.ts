import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, Package, PackageId } from "../definition/package.ts";

/** A named thing declared by a package: a resource, secret, service, connection, setting and so on. */
export interface Declaration {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The package-local declaration name. */
    readonly name: string;
}

/** A declaration qualified by its declaring package, as bindings, permissions and events store it. */
export const DeclarationReference = defineSchema(
    schema.object({
        /** The immutable identity of the declaring package. */
        packageId: PackageId,
        /** The package-local declaration name. */
        name: DeclarationName,
    }),
);
/** A declaration qualified by its declaring package. */
export type DeclarationReference = schema.Infer<typeof DeclarationReference>;

/** Reference a declaration by its package identity and name. */
export function reference(declaration: Declaration): DeclarationReference {
    return { packageId: declaration.package.id, name: declaration.name };
}
