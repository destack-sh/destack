import type { StackInputOf } from "@destack/object";
import type { price, product, productFeature } from "../object/index.ts";

/** A product with its prices and feature grants by name, as a catalog lists it. */
export type CatalogProduct = StackInputOf<typeof product> & {
    /** The product's prices by name. */
    readonly prices?: Readonly<Record<string, Omit<StackInputOf<typeof price>, "parent">>>;
    /** The product's feature grants by name. */
    readonly productFeatures?: Readonly<
        Record<string, Omit<StackInputOf<typeof productFeature>, "parent">>
    >;
};

/** Define a catalog of products as the stack collections of its products, prices and grants. */
export function defineCatalog(catalog: Readonly<Record<string, CatalogProduct>>): {
    readonly products: Readonly<Record<string, StackInputOf<typeof product>>>;
    readonly prices: Readonly<Record<string, StackInputOf<typeof price>>>;
    readonly productFeatures: Readonly<Record<string, StackInputOf<typeof productFeature>>>;
} {
    // split each product from its prices and grants, each child keyed under its product
    const entries = Object.entries(catalog);
    const products = entries.map(([key, entry]) => {
        const { prices: _prices, productFeatures: _features, ...declared } = entry;

        return [key, declared] as const;
    });
    const prices = entries.flatMap(([key, entry]) =>
        Object.entries(entry.prices ?? {}).map(
            ([name, declared]) => [`${key}-${name}`, { ...declared, parent: key }] as const,
        ),
    );
    const productFeatures = entries.flatMap(([key, entry]) =>
        Object.entries(entry.productFeatures ?? {}).map(
            ([name, declared]) => [`${key}-${name}`, { ...declared, parent: key }] as const,
        ),
    );

    return {
        products: Object.fromEntries(products),
        prices: Object.fromEntries(prices),
        productFeatures: Object.fromEntries(productFeatures),
    };
}
