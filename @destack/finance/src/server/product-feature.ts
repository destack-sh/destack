import { product, productFeature, ProductFeature } from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** Implement product features: a product grants only declared features of its seller's packages. */
export function implementProductFeature(metering: Metering): typeof productFeature {
    return productFeature.implement({
        declaration: {
            // apply after the product once the feature's package is published
            after: [product],
            resolve: async (_name, declared, plan) => {
                await metering.requirePublished(plan, declared.packageId);

                return declared;
            },
        },
        methods: {
            create: async (context, next) => {
                // record what the feature's declaration grants beside the grant
                const declared = await ProductFeature.requireGrant(
                    context.input,
                    context,
                    metering.reader(context, context.self, context.scope),
                    metering.publisher(context),
                );

                return next(context.with({ input: { ...context.input, ...declared } }));
            },
            update: async (context, next) => {
                // record it again, as a later release may grant the feature differently
                const declared = await ProductFeature.requireGrant(
                    { ...context.target, ...context.input },
                    context,
                    metering.reader(context, context.self, context.scope),
                    metering.publisher(context),
                );

                return next(context.with({ input: { ...context.input, ...declared } }));
            },
        },
    });
}
