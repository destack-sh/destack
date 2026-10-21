import type { StackInputOf } from "@destack/object";
import type {} from "@destack/stack/declare";
import type { price, product, productFeature, seller } from "../object/index.ts";

/** Add the sellers and products collections to stacks. */
declare module "@destack/stack/declare" {
    interface StackDefinition {
        /** The space account's seller, as its create method takes it. */
        readonly sellers?: Readonly<Record<string, StackInputOf<typeof seller>>>;
        /** The space account's products, each with its prices and feature grants by name. */
        readonly products?: Readonly<
            Record<
                string,
                StackInputOf<typeof product> & {
                    /** The product's prices by name. */
                    readonly prices?: Readonly<Record<string, StackInputOf<typeof price>>>;
                    /** The product's feature grants by name. */
                    readonly productFeatures?: Readonly<
                        Record<string, StackInputOf<typeof productFeature>>
                    >;
                }
            >
        >;
    }
}
