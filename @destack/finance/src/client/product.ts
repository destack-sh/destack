import { canonicalize, defineSchema, type Identifier, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { FeatureGrant, Recurring } from "../object/index.ts";
import { Amount, Currency } from "../rate/amount.ts";
import type { connect } from "./client.ts";

/** A finance client a seller publishes through. */
type FinanceClient = ReturnType<typeof connect>;

/** A fixed price a product declares: a plan's fee each period, or a one-time purchase. */
const PriceDefinition = schema.object({
    /** The key the seller looks the price up by, unique among its prices. */
    lookupKey: schema.string().min(1),
    /** The currency of the amount. */
    currency: Currency,
    /** The amount in minor units. */
    unitAmount: Amount,
    /** The usage each billing period includes, in minor units, usually the amount itself. */
    includedUsage: Amount,
    /** The billing period. */
    recurring: Recurring,
});
/** A fixed price a product declares. */
type PriceDefinition = schema.Infer<typeof PriceDefinition>;

/** The schema of a product as a seller declares it. */
const productDefinition = defineSchema(
    schema.object({
        /** The name buyers see, which the product is kept under. */
        name: schema.string().min(1),
        /** What buyers get. */
        description: schema.string().min(1),
        /** Whether accounts without a subscription to the seller get its grants, which a product without prices alone may be. */
        isDefault: schema.boolean().exactOptional(),
        /** The fixed prices, such as the fee in each currency, none for a default product. */
        prices: schema.array(PriceDefinition),
        /** The features granted, with their values or limits. */
        features: schema.array(FeatureGrant),
    }),
);
/** A product as a seller declares it. */
export type ProductDefinition = schema.Infer<typeof productDefinition>;

/** A product as a seller declares it: its fees with the usage each includes, and the features it grants. */
export const ProductDefinition = Object.assign(productDefinition, {
    /** Publish a product with its prices and features. */
    async publish(
        finance: FinanceClient,
        accountId: Identifier<"account">,
        definition: ProductDefinition,
    ): Promise<void> {
        // keep the product, then replace its changed prices
        const productId = await ProductDefinition.keep(finance, accountId, definition);
        for (const price of definition.prices) {
            await ProductDefinition.price(finance, accountId, productId, price);
        }

        // grant exactly its features
        await ProductDefinition.grant(finance, accountId, productId, definition.features);
    },

    /** Keep a product by name with the declared description, creating it once. */
    async keep(
        finance: FinanceClient,
        accountId: Identifier<"account">,
        definition: ProductDefinition,
    ): Promise<string> {
        // update a product published before when its terms changed
        const { name, description } = definition;
        const isDefault = definition.isDefault ?? false;
        const { items } = await finance.product.list({ accountId, where: { name } });
        const [kept] = items;
        if (kept !== undefined) {
            if (kept.description !== description || kept.isDefault !== isDefault) {
                await finance.product.update({
                    accountId,
                    requestId: RequestId.create(),
                    id: kept.id,
                    description,
                    isDefault,
                });
            }

            return kept.id;
        }

        // create it
        const created = await finance.product.create({
            accountId,
            requestId: RequestId.create(),
            name,
            description,
            isDefault,
        });

        return created.id;
    },

    /** Keep a price under its lookup key, retiring one with other terms for a new one. */
    async price(
        finance: FinanceClient,
        accountId: Identifier<"account">,
        productId: string,
        declared: PriceDefinition,
    ): Promise<void> {
        // keep a price with the same terms
        const { lookupKey, ...terms } = declared;
        const { items } = await finance.price.list({ accountId, where: { lookupKey } });
        const [kept] = items;
        const current =
            kept === undefined
                ? undefined
                : {
                      currency: kept.currency,
                      unitAmount: kept.unitAmount,
                      includedUsage: kept.includedUsage,
                      recurring: kept.recurring,
                  };
        if (kept?.parentId === productId && canonicalize(current) === canonicalize(terms)) {
            return;
        }

        // retire a price with other terms, as prices never change their amounts
        if (kept !== undefined) {
            await finance.price.update({
                accountId,
                requestId: RequestId.create(),
                id: kept.id,
                lookupKey: null,
                active: false,
            });
        }

        // create the price under the key
        await finance.price.create({
            accountId,
            requestId: RequestId.create(),
            parentId: productId,
            lookupKey,
            currency: terms.currency,
            unitAmount: terms.unitAmount,
            includedUsage: terms.includedUsage,
            recurring: terms.recurring,
        });
    },

    /** Grant exactly the declared features. */
    async grant(
        finance: FinanceClient,
        accountId: Identifier<"account">,
        productId: string,
        features: readonly FeatureGrant[],
    ): Promise<void> {
        // read the product's grants by feature
        const { items } = await finance.productFeature.list({
            accountId,
            where: { parentId: productId },
        });
        const kept = new Map(items.map((item) => [keyOf(item), item]));
        const declared = new Set(features.map(keyOf));

        // create a missing grant, and update one at another value
        for (const grant of features) {
            const row = kept.get(keyOf(grant));
            if (row === undefined) {
                await finance.productFeature.create({
                    accountId,
                    requestId: RequestId.create(),
                    parentId: productId,
                    ...grant,
                });
            } else if (canonicalize(row.value) !== canonicalize(grant.value)) {
                await finance.productFeature.update({
                    accountId,
                    requestId: RequestId.create(),
                    id: row.id,
                    value: grant.value,
                });
            }
        }

        // delete a grant no longer declared
        for (const row of items.filter((item) => !declared.has(keyOf(item)))) {
            await finance.productFeature.delete({
                accountId,
                requestId: RequestId.create(),
                id: row.id,
            });
        }
    },
});

/** Key a grant by its feature's package and name. */
function keyOf(grant: Pick<FeatureGrant, "packageId" | "feature">): string {
    return `${grant.packageId}/${grant.feature}`;
}
