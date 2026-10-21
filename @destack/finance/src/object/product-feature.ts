import { DeclarationReference } from "@destack/package/declare";
import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select, type Snapshot } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { PackageId, DeclarationName } from "@destack/package";
import { type JsonValue, schema } from "@destack/schema";
import { CallError } from "@destack/object";
import type { ReleaseReader } from "../release/reader.ts";
import { product } from "./product.ts";
import { type Publisher, Seller } from "./seller.ts";

/** A feature a product grants its buyers, with the value or limit of the grant. */
export const productFeature = defineObject(
    {
        name: "product-feature",
        plural: "productFeatures",
        scope: space,
        declarable: { from: "create" },
        nested: { in: product, delete: "cascade", receive: "sell" },
        fields: {
            /** The package declaring the feature. */
            packageId: field.string(PackageId),
            /** The feature's name in its package. */
            feature: field.string(DeclarationName),
            /** A static feature's value or a metered one's limit, unlimited without one. */
            value: field.json(schema.json()).optional(),
        },
        permissions: {
            read: through("parent", "read"),
            sell: through("parent", "sell"),
        },
        constraints: {
            unique: { kind: "index", on: ["parentId", "packageId", "feature"], unique: true },
        },
    },
    (method) => ({
        create: method.create("sell", { fields: ["packageId", "feature", "value"] }),
        update: method.update("sell", { fields: ["value"] }),
        delete: method.delete("sell"),
    }),
);
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
        reader: Pick<Snapshot, "read">,
        releases: ReleaseReader,
        publisher: Publisher,
    ): Promise<void> {
        // read the account selling the product
        const sold = await reader
            .read(product.table)
            .findFirst({ where: { id: product.identifier(grant.parentId) } });
        if (sold === undefined) {
            throw new CallError("NOT_FOUND", { message: `no product ${grant.parentId}` });
        }

        // refuse a feature of a package another account publishes
        const reference = DeclarationReference.parse({
            packageId: grant.packageId,
            name: grant.feature,
        });
        const feature = await releases.feature(reference);
        await Seller.requirePublisher(sold.accountId, feature.package, publisher, reader);

        // refuse a value the feature's declaration rejects
        feature.requireGrant(grant.value ?? null);
    },
};
