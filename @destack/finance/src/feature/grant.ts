import { PackageId, DeclarationName } from "@destack/package";
import { schema } from "@destack/schema";
import { ENTITLEMENT_RESOURCES, FEATURE_KINDS } from "./allowance.ts";

/** A product's grant of a feature, as subscription items and purchases store it. */
export const FeatureGrant = schema.object({
    /** The package declaring the feature. */
    packageId: PackageId,
    /** The feature's name in its package. */
    feature: DeclarationName,
    /** The value of a static feature, or the limit of a metered one, null for none. */
    value: schema.json().nullable(),
    /** What the feature grants, as its declaration said when the grant was made. */
    kind: schema.enum(FEATURE_KINDS),
    /** The platform resource the feature allows, null for a package's own feature. */
    resource: schema.enum(ENTITLEMENT_RESOURCES).nullable(),
});
/** A product's grant of a feature. */
export type FeatureGrant = schema.Infer<typeof FeatureGrant>;
