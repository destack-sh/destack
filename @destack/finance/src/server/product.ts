import { Price, product, seller } from "../object/index.ts";

/** Implement products: a default product takes no prices, and an undeclared one is archived. */
export function implementProduct(): typeof product {
    return product.implement({
        methods: {
            update: {
                authorize: (context) =>
                    Price.requireNone({ ...context.target, ...context.input }, context),
            },
        },
        declaration: {
            // apply after the seller its prices check against
            after: [seller],
            // archive a product its stack stops declaring
            retire: "archived",
        },
    });
}
