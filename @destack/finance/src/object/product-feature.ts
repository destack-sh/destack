import { through } from "@destack/access";
import { account } from "@destack/account/object";
import { uniqueIndex, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { product } from "./product.ts";
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

/** A feature a product grants its buyers, with the value or limit of the grant. */
export const productFeature = defineObject({
    name: "product-feature",
    plural: "productFeatures",
    scope: account,
    nested: { in: product, delete: "cascade", receive: "sell" },
    fields: {
        /** The package declaring the feature. */
        packageId: field.string(PackageId),
        /** The feature's name in its package. */
        feature: field.string(CatalogName),
        /** The value of a static feature, or the limit of a metered one, unlimited without one. */
        value: field.json(schema.json()).optional(),
    },
    permissions: {
        read: through("parent", "read"),
        sell: through("parent", "sell"),
    },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("sell", { fields: ["packageId", "feature", "value"] }),
        update: method.update("sell", { fields: ["value"] }),
        delete: method.delete("sell"),
    }),
    constraints: (entry) => [
        uniqueIndex("product_feature_unique").on(entry.parentId, entry.packageId, entry.feature),
    ],
});
/** A persisted product feature. */
export type ProductFeature = Select<typeof productFeature.table>;
