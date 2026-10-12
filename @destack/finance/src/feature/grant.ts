import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { CatalogName } from "../catalog/reference.ts";

/** A product's grant of a feature, as subscription items and purchases keep it. */
export const FeatureGrant = defineSchema(
    schema.object({
        /** The package declaring the feature. */
        packageId: PackageId,
        /** The feature's name in its package. */
        feature: CatalogName,
        /** The value of a static feature, or the limit of a metered one, null for none. */
        value: schema.json().nullable(),
    }),
);
/** A product's grant of a feature. */
export type FeatureGrant = schema.Infer<typeof FeatureGrant>;
