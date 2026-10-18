import { product, productFeature, ProductFeature } from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** Implement product features: a product grants only declared features of its seller's packages. */
export function implementProductFeature(metering: Metering): typeof productFeature {
    const { publisher } = metering;

    return productFeature.implement({
        declaration: { after: [product] },
        methods: {
            create: {
                authorize: (context) =>
                    ProductFeature.requireGrant(
                        context.input,
                        context,
                        metering.reader(context, context.self, context.scope),
                        publisher,
                    ),
            },
            update: {
                authorize: (context) =>
                    ProductFeature.requireGrant(
                        { ...context.target, ...context.input },
                        context,
                        metering.reader(context, context.self, context.scope),
                        publisher,
                    ),
            },
        },
    });
}
