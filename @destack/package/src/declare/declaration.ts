import { schema, type JsonObject } from "@destack/schema";
import { DeclarationName, PackageVersion, PackageId } from "../definition/package.ts";
import type { Kind } from "../definition/kind.ts";

/** A declared thing of a package with a name: a resource, secret, service, setting and so on. */
export interface Declaration {
    /** The declaring package, supplied by the module transform. */
    readonly package: PackageVersion;
    /** The package-local declaration name. */
    readonly name: string;
}

/** The state a resource declaration requires of its resource. */
export type ResourceState = JsonObject;

/** A resource a package requires, described by its kind and the state it must have. */
export interface ResourceRequirement extends Declaration {
    /** The resource kind: a provisioned one, such as database, or a bound one, such as secret. */
    readonly kind: Kind;
    /** Describe the state the resource must have. */
    readonly state: () => ResourceState;
}

/** The schema of a declaration qualified by its declaring package. */
const declarationReferenceSchema = schema.object({
    /** The immutable identity of the declaring package. */
    packageId: PackageId,
    /** The package-local declaration name. */
    name: DeclarationName,
});
/** A declaration qualified by its declaring package. */
export type DeclarationReference = schema.Infer<typeof declarationReferenceSchema>;

/** A declaration qualified by its package, as bindings, permissions and events store it. */
export const DeclarationReference = Object.assign(declarationReferenceSchema, {
    /** Reference a declaration by its package identity and name. */
    of(declaration: Declaration): DeclarationReference {
        return { packageId: declaration.package.id, name: declaration.name };
    },

    /** Key a reference by its package and name, as `<packageId>/<name>`. */
    key(reference: DeclarationReference): string {
        return `${reference.packageId}/${reference.name}`;
    },

    /** Read a reference from its key, refusing a malformed one. */
    ofKey(key: string): DeclarationReference {
        const separator = key.indexOf("/");

        return declarationReferenceSchema.parse({
            packageId: key.slice(0, separator),
            name: key.slice(separator + 1),
        });
    },
});
