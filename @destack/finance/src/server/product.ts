import { JsonValue } from "@destack/schema";
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
            // offer a declared product again, unless it says otherwise
            resolve: async (_name, declared) => ({ active: true, ...JsonValue.object(declared) }),
            // archive a product its stack stops declaring
            retire: "archived",
        },
    });
}
