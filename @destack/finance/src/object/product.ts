import { resource, through, union } from "@destack/access";
import { account } from "@destack/account/object";
import { uniqueIndex, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { FeatureName } from "../feature/feature.ts";

/** The longest product name, one line of text. */
const NAME_LENGTH = 250;

/** The longest product description, a few paragraphs. */
const DESCRIPTION_LENGTH = 4000;

/** A good or service a seller's account offers, priced by its prices and granting its features. */
export const product = defineObject({
    name: "product",
    plural: "products",
    scope: account,
    fields: {
        /** The name buyers see. */
        name: field.string(schema.string().min(1).max(NAME_LENGTH)),
        /** What buyers get. */
        description: field.string(schema.string().min(1).max(DESCRIPTION_LENGTH)),
        /** Whether buyers may buy it. */
        active: field.boolean().default(true),
    },
    attributes: { active: "boolean" },
    permissions: {
        read: union(through("account", "read"), resource({ active: true })),
        sell: through("account", "manage"),
    },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("sell", { fields: ["name", "description", "active"] }),
        update: method.update("sell", { fields: ["name", "description", "active"] }),
    }),
});
/** A persisted product. */
export type Product = Select<typeof product.table>;

/** A product's grant of a feature, as subscription items and purchases keep it. */
export const FeatureGrant = defineSchema(
    schema.object({
        /** The package declaring the feature. */
        packageId: PackageId,
        /** The feature's name in its package. */
        feature: FeatureName,
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
        feature: field.string(FeatureName),
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
