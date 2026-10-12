import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select, type Snapshot } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { type JsonValue, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Catalog } from "../catalog/catalog.ts";
import { CatalogName, CatalogReference } from "../catalog/reference.ts";
import { product } from "./product.ts";
import { type Publisher, Seller } from "./seller.ts";

/** A feature a product grants its buyers, with the value or limit of the grant. */
export const productFeature = defineObject({
    name: "product-feature",
    plural: "productFeatures",
    scope: space,
    declarable: { from: "create" },
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
    constraints: {
        unique: { kind: "index", on: ["parentId", "packageId", "feature"], unique: true },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    create: method.create("sell", { fields: ["packageId", "feature", "value"] }),
    update: method.update("sell", { fields: ["value"] }),
    delete: method.delete("sell"),
}));
/** A persisted product feature. */
export type ProductFeature = Select<typeof productFeature.table>;

/** The features products grant. */
export const ProductFeature = {
    /** Require a seller's grant of a feature it publishes on one of its products. */
    async requireGrant(
        grant: {
            readonly parentId: string;
            readonly packageId: string;
            readonly feature: string;
            readonly value?: JsonValue;
        },
        snapshot: Snapshot,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        publisher: Publisher,
    ): Promise<void> {
        // read the account selling the product
        const sold = await snapshot.row(product.table, { id: product.identifier(grant.parentId) });
        if (sold === null) {
            throw new ServiceError("NOT_FOUND", { message: `no product ${grant.parentId}` });
        }

        // refuse a feature of a package another account publishes
        const reference = CatalogReference.parse({
            packageId: grant.packageId,
            name: grant.feature,
        });
        const declared = await catalog(reference.packageId);
        await Seller.requirePublisher(snapshot, sold.accountId, declared.package, publisher);

        // refuse a value the feature's declaration rejects
        declared.feature(reference).requireGrant(grant.value ?? null);
    },
};
