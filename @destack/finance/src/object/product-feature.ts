import { DeclarationReference } from "@destack/package/declare";
import { FinanceError } from "../error/error.ts";
import { through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import type { Snapshot } from "@destack/db/database";
import { defineObject, field, nested } from "@destack/object";
import { PackageId, DeclarationName } from "@destack/package";
import { type JsonValue, schema } from "@destack/schema";
import type { ReleaseReader } from "../release/reader.ts";
import { ENTITLEMENT_RESOURCES, FEATURE_KINDS } from "../feature/allowance.ts";
import { product } from "./product.ts";
import { type Publisher, Seller } from "./seller.ts";
import type {} from "@destack/package/import-meta";
import { argument, plural, t } from "@destack/locale";

/** A feature a product grants its buyers, with the value or limit of the grant. */
export const productFeature = defineObject(
    {
        name: "product-feature",
        plural: "productFeatures",
        title: t`${plural(argument("count"), { one: "Product feature", other: "Product features" })}`,
        scope: space,
        fields: {
            /** The package declaring the feature. */
            packageId: field.string(PackageId),
            /** The feature's name in its package. */
            feature: field.string(DeclarationName),
            /** A static feature's value or a metered one's limit, unlimited without one. */
            value: field.json(schema.json()).optional(),
            /** What the feature grants, as its declaration says when the grant is made. */
            kind: field.enum(FEATURE_KINDS),
            /** The platform resource the feature allows, absent for a package's own feature. */
            resource: field.enum(ENTITLEMENT_RESOURCES).optional(),
        },
        constraints: {
            unique: { kind: "index", on: ["parentId", "packageId", "feature"], unique: true },
        },
        access: {
            permissions: {
                read: through("parent", "read"),
                sell: through("parent", "sell"),
            },
        },
        traits: [nested({ in: product, delete: "cascade", add: "sell" })],
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
    /** Require a seller's grant of a feature it publishes, reading what its declaration grants. */
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
    ): Promise<Pick<ProductFeature, "kind" | "resource">> {
        // read the account selling the product
        const sold = await reader
            .read(product.table)
            .findFirst({ where: { id: product.identifier(grant.parentId) } });
        if (sold === undefined) {
            throw FinanceError.of("PRODUCT_NOT_FOUND", `product '${grant.parentId}' not found`);
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

        return { kind: feature.definition.kind, resource: feature.definition.resource ?? null };
    },
};
