import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";

/** A package-local name of a feature, meter or SKU, kept across releases. */
export const CatalogName = defineSchema(
    schema.string().regex(/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$(?![\s\S])/u),
);

/** The schema of a feature, meter or SKU by its package and name. */
const catalogReferenceSchema = defineSchema(
    schema.object({
        /** The declaring package. */
        packageId: PackageId,
        /** The package-local name. */
        name: CatalogName,
    }),
);
/** The identity of a feature, meter or SKU. */
export type CatalogReference = schema.Infer<typeof catalogReferenceSchema>;

/** The identity of a feature, meter or SKU across package renames and releases. */
export const CatalogReference = Object.assign(catalogReferenceSchema, {
    /** Key a declaration by its package and name. */
    key(reference: CatalogReference): string {
        return `${reference.packageId}/${reference.name}`;
    },

    /** Read a reference from its key, refusing a malformed one. */
    of(key: string): CatalogReference {
        const separator = key.indexOf("/");

        return {
            packageId: PackageId.parse(key.slice(0, separator)),
            name: CatalogName.parse(key.slice(separator + 1)),
        };
    },
});
